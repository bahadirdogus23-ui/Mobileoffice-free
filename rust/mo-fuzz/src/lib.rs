//! Helpers for the bridges' robustness tests: thousands of damaged, cut and hostile variants of
//! valid files are fed to the code the bridges own, and each must come back (an error is fine)
//! without a panic, within a time limit, and within a memory limit.
//!
//! Everything is deterministic: a case is the same input on every run, so a failure is
//! reproduced by running the same test again. `MO_FUZZ_ITERS=n` runs `n` cases per target instead
//! of the default (`MO_FUZZ_SEED` changes the inputs); `MO_FUZZ_VERBOSE=1` prints each case and
//! keeps the input being tried in `<temp>/mo-fuzz/<target>-current.bin`, for the failures that
//! end the process (a stack overflow, an allocation past the hard limit).
//!
//! The memory limit needs the counting allocator in the test binary:
//!
//! ```ignore
//! #[cfg(test)]
//! #[global_allocator]
//! static ALLOC: mo_fuzz::CountingAlloc = mo_fuzz::CountingAlloc;
//! ```

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::panic::AssertUnwindSafe;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A small xorshift generator: the same seed gives the same inputs everywhere.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// A number in `0..n` (0 when `n` is 0).
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next_u64() % n as u64) as usize }
    }

    /// True once in `n` times.
    pub fn one_in(&mut self, n: usize) -> bool {
        self.below(n) == 0
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

/// How many cases a target runs: `MO_FUZZ_ITERS` when set, else `default`.
pub fn iterations(default: usize) -> usize {
    std::env::var("MO_FUZZ_ITERS").ok().and_then(|v| v.trim().parse().ok()).unwrap_or(default)
}

fn base_seed() -> u64 {
    std::env::var("MO_FUZZ_SEED").ok().and_then(|v| v.trim().parse().ok()).unwrap_or(0x4D4F_4655_5A5A)
}

fn verbose() -> bool {
    std::env::var("MO_FUZZ_VERBOSE").is_ok_and(|v| !v.is_empty() && v != "0")
}

const BYTES: &[u8] = &[
    0, 1, 0x7F, 0x80, 0xFF, 0xFE, 0xC3, 0xC4, 0xC5, 0xDD, 0xFD, 0xF0, b'<', b'>', b'/', b'"', b'\'', b'%', b'&', b';', b':', b'=', b'\n', b'\r',
    b'\\', b'{', b'}', b'.', b' ', b'0', b'9', b'-',
];

const INTS: &[u32] =
    &[0, 1, 2, 0x7F, 0x80, 0xFF, 0x100, 0x7FFF, 0x8000, 0xFFFF, 0x1_0000, 0x00FF_FFFF, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFE, 0xFFFF_FFFF];

/// Text tokens most text formats here react to.
pub const TEXT_TOKENS: &[&[u8]] = &[
    b"\r\n",
    b"\n",
    b" ",
    b"\t",
    b"=",
    b":",
    b";",
    b",",
    b"\"",
    b"'",
    b"<",
    b">",
    b"</",
    b"/>",
    b"&amp;",
    b"&#",
    b"%%",
    b"%",
    b"\\",
    b"{",
    b"}",
    b"\xc4\xb0",
    b"\xc5\x9f",
    b"\xdd",
    b"\xfe",
    b"\xff\xfe",
    b"\xef\xbb\xbf",
    b"\xf0\x9f\x98\x80",
    b"\xc3",
    b"\x00",
];

/// XML tokens: deep nesting, entities, odd attributes.
pub const XML_TOKENS: &[&[u8]] = &[
    b"<a>",
    b"</a>",
    b"<a/>",
    b"<![CDATA[",
    b"]]>",
    b"<!--",
    b"-->",
    b"<?xml version=\"1.0\"?>",
    b"<!DOCTYPE x [<!ENTITY e \"eeeeeeeeee\">]>",
    b"&e;",
    b"&lt;",
    b"&#x0;",
    b"&#1114112;",
    b" x=\"1\"",
    b" xmlns=\"urn:x\"",
    b" xmlns:p=\"\"",
    b"<p:q>",
    b"=\"",
    b"\"",
    b"'",
    b"<",
    b">",
];

/// A span `(start, len)` of at most `max` bytes inside `len` bytes.
fn span(rng: &mut Rng, len: usize, max: usize) -> (usize, usize) {
    let start = rng.below(len + 1);
    let n = rng.below((len - start).min(max) + 1);
    (start, n)
}

/// `seed` changed in one to four places: bits flipped, bytes and integers set to edge values,
/// cut short, pieces deleted, copied, inserted or repeated, tokens from `dict` put in.
pub fn mutate(rng: &mut Rng, seed: &[u8], dict: &[&[u8]]) -> Vec<u8> {
    let mut out = seed.to_vec();
    for _ in 0..1 + rng.below(4) {
        mutate_once(rng, &mut out, dict);
    }
    out
}

fn mutate_once(rng: &mut Rng, data: &mut Vec<u8>, dict: &[&[u8]]) {
    let len = data.len();
    match rng.below(13) {
        0 if len > 0 => {
            let i = rng.below(len);
            data[i] ^= 1 << rng.below(8);
        }
        1 if len > 0 => {
            let i = rng.below(len);
            data[i] = *rng.pick(BYTES);
        }
        2 if len > 0 => {
            let i = rng.below(len);
            data[i] = rng.next_u64() as u8;
        }
        3 if len >= 4 => {
            let v = *rng.pick(INTS);
            let i = rng.below(len - 3);
            let bytes = match rng.below(4) {
                0 => v.to_le_bytes().to_vec(),
                1 => v.to_be_bytes().to_vec(),
                2 => (v as u16).to_le_bytes().to_vec(),
                _ => (v as u16).to_be_bytes().to_vec(),
            };
            data[i..i + bytes.len()].copy_from_slice(&bytes);
        }
        4 => data.truncate(rng.below(len + 1)),
        5 => {
            let max = if rng.one_in(8) { len } else { 64 };
            let (start, n) = span(rng, len, max);
            data.drain(start..start + n);
        }
        6 => {
            let (start, n) = span(rng, len, 256);
            let piece = data[start..start + n].to_vec();
            let at = rng.below(len + 1);
            data.splice(at..at, piece);
        }
        7 => {
            let at = rng.below(len + 1);
            let piece: Vec<u8> = (0..1 + rng.below(16)).map(|_| rng.next_u64() as u8).collect();
            data.splice(at..at, piece);
        }
        8 | 9 => {
            let token: &[u8] = if dict.is_empty() { rng.pick(TEXT_TOKENS) } else { rng.pick(dict) };
            let at = rng.below(len + 1);
            data.splice(at..at, token.iter().copied());
        }
        10 | 11 => {
            // A piece repeated: nesting, long runs, many records. Now and then a lot of it, to
            // show work that grows faster than the input.
            let piece: Vec<u8> = if !dict.is_empty() && rng.one_in(2) {
                rng.pick(dict).to_vec()
            } else {
                let (start, n) = span(rng, len, 32);
                data[start..start + n].to_vec()
            };
            if piece.is_empty() {
                return;
            }
            let times = if rng.one_in(24) { 1000 + rng.below(20_000) } else { 2 + rng.below(64) };
            let times = times.min((256 * 1024) / piece.len()).max(1);
            let at = rng.below(len + 1);
            data.splice(at..at, piece.iter().copied().cycle().take(piece.len() * times));
        }
        12 if len > 1 => {
            let (from, n) = span(rng, len, 64);
            let to = rng.below(len - n + 1);
            let piece = data[from..from + n].to_vec();
            data[to..to + n].copy_from_slice(&piece);
        }
        _ => {
            let token: &[u8] = if dict.is_empty() { rng.pick(TEXT_TOKENS) } else { rng.pick(dict) };
            data.extend_from_slice(token);
        }
    }
}

/// The entries of a zip as (name, bytes).
pub type Entries = Vec<(String, Vec<u8>)>;

/// `entries` with one change: an entry's bytes mutated (most often), its name changed, an entry
/// dropped, doubled or added.
pub fn mutate_entries(rng: &mut Rng, entries: &[(String, Vec<u8>)], dict: &[&[u8]]) -> Entries {
    let mut out = entries.to_vec();
    if out.is_empty() {
        out.push(("x".into(), mutate(rng, b"", dict)));
        return out;
    }
    let i = rng.below(out.len());
    match rng.below(10) {
        0 => {
            let name = mutate(rng, out[i].0.as_bytes(), &[b"/", b"../", b"\\", b"\xc4\xb0", b"%2F", b".."]);
            out[i].0 = String::from_utf8_lossy(&name).into_owned();
        }
        1 => {
            out.remove(i);
        }
        2 => {
            let e = out[i].clone();
            out.push(e);
        }
        _ => {
            let data = mutate(rng, &out[i].1, dict);
            out[i].1 = data;
        }
    }
    out
}

/// A zip of `entries`, mutated by [`mutate_entries`]; now and then its raw bytes are mutated too.
pub fn mutate_zip(rng: &mut Rng, entries: &[(String, Vec<u8>)], dict: &[&[u8]]) -> Vec<u8> {
    let changed = mutate_entries(rng, entries, dict);
    let zip = stored_zip(&changed);
    if rng.one_in(5) { mutate(rng, &zip, &[]) } else { zip }
}

/// The CRC-32 (IEEE) zip uses.
pub fn crc32(data: &[u8]) -> u32 {
    static TABLE: std::sync::OnceLock<[u32; 256]> = std::sync::OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut t = [0u32; 256];
        for (i, slot) in t.iter_mut().enumerate() {
            let mut c = i as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            }
            *slot = c;
        }
        t
    });
    !data.iter().fold(!0u32, |c, &b| table[((c ^ u32::from(b)) & 0xFF) as usize] ^ (c >> 8))
}

/// A zip holding `entries` uncompressed (any names, duplicates included).
pub fn stored_zip(entries: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in entries {
        let offset = out.len() as u32;
        let crc = crc32(data);
        let header = |sig: u32, central: bool| {
            let mut h = sig.to_le_bytes().to_vec();
            if central {
                h.extend_from_slice(&20u16.to_le_bytes());
            }
            h.extend_from_slice(&20u16.to_le_bytes()); // version needed
            h.extend_from_slice(&0x0800u16.to_le_bytes()); // UTF-8 names
            h.extend_from_slice(&0u16.to_le_bytes()); // stored
            h.extend_from_slice(&0u32.to_le_bytes()); // time and date
            h.extend_from_slice(&crc.to_le_bytes());
            h.extend_from_slice(&(data.len() as u32).to_le_bytes());
            h.extend_from_slice(&(data.len() as u32).to_le_bytes());
            h.extend_from_slice(&(name.len() as u16).to_le_bytes());
            h.extend_from_slice(&0u16.to_le_bytes()); // extra
            if central {
                h.extend_from_slice(&[0; 6]); // comment, disk, internal attributes
                h.extend_from_slice(&0u32.to_le_bytes()); // external attributes
                h.extend_from_slice(&offset.to_le_bytes());
            }
            h.extend_from_slice(name.as_bytes());
            h
        };
        out.extend(header(0x0403_4B50, false));
        out.extend_from_slice(data);
        central.extend(header(0x0201_4B50, true));
    }
    let cd_offset = out.len() as u32;
    let n = entries.len().min(0xFFFF) as u16;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4B50u32.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&n.to_le_bytes());
    out.extend_from_slice(&n.to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

/// Limits for one target.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// The longest one case may take (debug build).
    pub per_case: Duration,
    /// The most one case may allocate on top of what was allocated before it, besides
    /// `per_input_byte` times the input's length.
    pub memory: usize,
    pub per_input_byte: usize,
}

impl Default for Limits {
    fn default() -> Limits {
        let secs = std::env::var("MO_FUZZ_CASE_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(20);
        Limits { per_case: Duration::from_secs(secs), memory: 384 << 20, per_input_byte: 64 }
    }
}

/// Past this a case's allocations fail (and the process ends): a memory bomb must not take the
/// machine the tests run on with it.
const HARD_MEMORY: isize = 3 << 30;

enum Msg {
    Start(usize),
    Fail(String),
    Done { cases: usize, slowest: Duration, peak: usize },
}

fn dump_path(target: &str, what: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("mo-fuzz");
    let _ = std::fs::create_dir_all(&dir);
    dir.join(format!("{target}-{what}.bin"))
}

/// Runs `cases` cases of target `name` with the default [`Limits`]: `make(rng, case)` makes an
/// input, `check(input)` feeds it to the code under test. Panics (failing the test) on the first
/// case that panics, outlasts the time limit or allocates past the memory limit, after saving
/// that input under `<temp>/mo-fuzz/`.
pub fn run<M, C>(name: &str, cases: usize, make: M, check: C)
where
    M: FnMut(&mut Rng, usize) -> Vec<u8> + Send + 'static,
    C: Fn(&[u8]) + Send + 'static,
{
    run_with(name, cases, Limits::default(), make, check)
}

/// [`run`] with other limits.
pub fn run_with<M, C>(name: &str, cases: usize, limits: Limits, mut make: M, check: C)
where
    M: FnMut(&mut Rng, usize) -> Vec<u8> + Send + 'static,
    C: Fn(&[u8]) + Send + 'static,
{
    let started = Instant::now();
    let current: Arc<Mutex<Vec<u8>>> = Arc::default();
    let (tx, rx) = mpsc::channel();
    let target = name.to_string();
    let shared = Arc::clone(&current);
    let verbose = verbose();
    let seed = base_seed() ^ fnv(name.as_bytes());
    std::thread::Builder::new()
        .name(format!("fuzz-{name}"))
        .stack_size(8 << 20)
        .spawn(move || {
            let mut rng = Rng::new(seed);
            let (mut slowest, mut peak) = (Duration::ZERO, 0usize);
            for case in 0..cases {
                let input = make(&mut rng, case);
                if let Ok(mut c) = shared.lock() {
                    c.clear();
                    c.extend_from_slice(&input);
                }
                if verbose {
                    println!("{target}: case {case} ({} bytes)", input.len());
                    let _ = std::fs::write(dump_path(&target, "current"), &input);
                }
                let _ = tx.send(Msg::Start(case));
                let soft = limits.memory.saturating_add(limits.per_input_byte.saturating_mul(input.len()));
                let base = begin_counting(HARD_MEMORY);
                let t = Instant::now();
                let result = std::panic::catch_unwind(AssertUnwindSafe(|| check(&input)));
                let took = t.elapsed();
                let used = end_counting(base);
                slowest = slowest.max(took);
                peak = peak.max(used);
                let fail = match result {
                    Err(p) => Some(format!("panicked: {}", panic_text(&p))),
                    Ok(()) if used > soft => Some(format!("allocated {} MB at its peak (limit {} MB)", used >> 20, soft >> 20)),
                    Ok(()) if took > limits.per_case => Some(format!("took {took:?} (limit {:?})", limits.per_case)),
                    Ok(()) => None,
                };
                if let Some(why) = fail {
                    let path = dump_path(&target, &case.to_string());
                    let _ = std::fs::write(&path, &input);
                    let _ = tx.send(Msg::Fail(format!("{target}: case {case} ({} bytes) {why}; input saved to {}", input.len(), path.display())));
                    return;
                }
            }
            let _ = tx.send(Msg::Done { cases, slowest, peak });
        })
        .expect("spawn the fuzz worker");
    let mut case = 0;
    loop {
        match rx.recv_timeout(limits.per_case + Duration::from_secs(5)) {
            Ok(Msg::Start(c)) => case = c,
            Ok(Msg::Fail(why)) => panic!("{why}"),
            Ok(Msg::Done { cases, slowest, peak }) => {
                println!(
                    "{name}: {cases} cases in {:.1} s (slowest {:.0} ms, peak {} MB)",
                    started.elapsed().as_secs_f64(),
                    slowest.as_secs_f64() * 1000.0,
                    peak >> 20
                );
                return;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let path = dump_path(name, &case.to_string());
                if let Ok(c) = current.lock() {
                    let _ = std::fs::write(&path, &*c);
                }
                panic!("{name}: case {case} hangs (over {:?}); input saved to {}", limits.per_case, path.display());
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => panic!("{name}: the worker ended at case {case}"),
        }
    }
}

fn fnv(b: &[u8]) -> u64 {
    b.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &c| (h ^ u64::from(c)).wrapping_mul(0x100_0000_01b3))
}

fn panic_text(p: &Box<dyn std::any::Any + Send>) -> String {
    p.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| p.downcast_ref::<String>().cloned()).unwrap_or_else(|| "(no message)".into())
}

/// Calls `f` and returns whether it panicked, with the panic's message: for the bridges'
/// `catch_unwind` guards, whose tests feed them inputs known to panic in an engine.
pub fn panics(f: impl FnOnce()) -> Option<String> {
    std::panic::catch_unwind(AssertUnwindSafe(f)).err().map(|p| panic_text(&p))
}

// The counting allocator: per-thread bytes in use and their peak, with a hard limit.

struct Counts {
    current: Cell<isize>,
    peak: Cell<isize>,
    limit: Cell<isize>,
}

thread_local! {
    static COUNTS: Counts = const { Counts { current: Cell::new(0), peak: Cell::new(0), limit: Cell::new(0) } };
}

fn begin_counting(limit: isize) -> isize {
    COUNTS.with(|c| {
        c.peak.set(c.current.get());
        c.limit.set(c.current.get().saturating_add(limit));
        c.current.get()
    })
}

fn end_counting(base: isize) -> usize {
    COUNTS.with(|c| {
        c.limit.set(0);
        (c.peak.get() - base).max(0) as usize
    })
}

fn reserve(n: usize) -> bool {
    COUNTS
        .try_with(|c| {
            let now = c.current.get().saturating_add(n as isize);
            if c.limit.get() > 0 && now > c.limit.get() {
                return false;
            }
            c.current.set(now);
            if now > c.peak.get() {
                c.peak.set(now);
            }
            true
        })
        .unwrap_or(true)
}

fn release(n: usize) {
    let _ = COUNTS.try_with(|c| c.current.set(c.current.get().saturating_sub(n as isize)));
}

/// The system allocator, counting what each thread has in use (see the crate documentation).
pub struct CountingAlloc;

// SAFETY: every call is passed on to the system allocator unchanged; only counters change.
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if !reserve(layout.size()) {
            return std::ptr::null_mut();
        }
        // SAFETY: the caller's contract is passed on.
        let p = unsafe { System.alloc(layout) };
        if p.is_null() {
            release(layout.size());
        }
        p
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if !reserve(layout.size()) {
            return std::ptr::null_mut();
        }
        // SAFETY: the caller's contract is passed on.
        let p = unsafe { System.alloc_zeroed(layout) };
        if p.is_null() {
            release(layout.size());
        }
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the caller's contract is passed on.
        unsafe { System.dealloc(ptr, layout) };
        release(layout.size());
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let old = layout.size();
        if new_size > old && !reserve(new_size - old) {
            return std::ptr::null_mut();
        }
        // SAFETY: the caller's contract is passed on.
        let p = unsafe { System.realloc(ptr, layout, new_size) };
        if p.is_null() {
            if new_size > old {
                release(new_size - old);
            }
        } else if new_size < old {
            release(old - new_size);
        }
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_inputs() {
        let seed = b"<a x=\"1\">text</a>".to_vec();
        let a: Vec<Vec<u8>> = {
            let mut r = Rng::new(7);
            (0..50).map(|_| mutate(&mut r, &seed, XML_TOKENS)).collect()
        };
        let b: Vec<Vec<u8>> = {
            let mut r = Rng::new(7);
            (0..50).map(|_| mutate(&mut r, &seed, XML_TOKENS)).collect()
        };
        assert_eq!(a, b);
        assert!(a.iter().any(|m| *m != seed));
    }

    #[test]
    fn crc_and_zip_layout() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        let z = stored_zip(&[("a.txt".into(), b"hi".to_vec())]);
        assert!(z.starts_with(b"PK\x03\x04"));
        assert_eq!(&z[z.len() - 22..z.len() - 18], b"PK\x05\x06");
    }

    #[test]
    fn a_panic_is_reported() {
        assert_eq!(panics(|| panic!("boom")).as_deref(), Some("boom"));
        assert_eq!(panics(|| ()), None);
    }
}
