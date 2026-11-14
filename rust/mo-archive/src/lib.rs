//! 7z archives (7-Zip, often sent instead of ZIP): what is inside them, and one entry's bytes to
//! open in the app. ZIP stays on Android's own reader in the app.

use std::path::Path;

/// One file or folder inside an archive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Its path inside the archive, with `/` between folders.
    pub path: String,
    /// Its unpacked size in bytes.
    pub size: u64,
    pub is_dir: bool,
}

/// What can go wrong reading an archive.
#[derive(Debug, PartialEq, Eq)]
pub enum ArchiveError {
    /// Not a 7z archive, or a damaged one.
    Unreadable(String),
    /// The archive is locked with a password.
    Encrypted,
    /// No entry has that path.
    NoEntry(String),
    /// The entry unpacks to more than the app opens.
    TooBig(u64),
}

impl std::fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArchiveError::Unreadable(m) => write!(f, "{m}"),
            ArchiveError::Encrypted => write!(f, "the archive is password protected"),
            ArchiveError::NoEntry(p) => write!(f, "no entry {p}"),
            ArchiveError::TooBig(n) => write!(f, "the entry is {n} bytes, too big to open"),
        }
    }
}

fn unreadable(e: sevenz_rust2::Error) -> ArchiveError {
    let text = e.to_string();
    if text.to_ascii_lowercase().contains("password") || text.to_ascii_lowercase().contains("encrypt") {
        ArchiveError::Encrypted
    } else {
        ArchiveError::Unreadable(text)
    }
}

/// Whether `head` (the first bytes of a file) starts a 7z archive.
pub fn is_7z(head: &[u8]) -> bool {
    head.starts_with(&[b'7', b'z', 0xBC, 0xAF, 0x27, 0x1C])
}

/// The most memory (KiB) the reader may take for the archive's header and for each decoder's
/// dictionary or model. Without it a damaged or hostile archive that declares a 4 GB dictionary
/// makes the reader ask for 4 GB, and a failed allocation ends the app instead of an error.
/// 7-Zip's "Ultra" level uses 64 MB dictionaries; larger ones are chosen by hand.
const MEMORY_LIMIT_KB: usize = 256 * 1024;

/// The archive at `path`'s header (entries, blocks) and the open file.
fn open(path: &Path) -> Result<(sevenz_rust2::Archive, std::fs::File), ArchiveError> {
    let mut file = std::fs::File::open(path).map_err(|e| ArchiveError::Unreadable(e.to_string()))?;
    let archive = sevenz_rust2::Archive::read_with_memory_limit(&mut file, &sevenz_rust2::Password::empty(), MEMORY_LIMIT_KB).map_err(unreadable)?;
    Ok((archive, file))
}

/// The entries of the 7z archive at `path`, in the archive's order.
pub fn list(path: &Path) -> Result<Vec<Entry>, ArchiveError> {
    let (archive, _) = open(path)?;
    Ok(archive
        .files
        .iter()
        .filter(|e| !e.is_anti_item)
        .map(|e| Entry { path: e.name.replace('\\', "/"), size: e.size, is_dir: e.is_directory })
        .collect())
}

/// The bytes of entry `name` of the 7z archive at `path`, refused above `max` bytes.
///
/// Only the block holding the entry is decoded. In a solid block the entries before it are
/// decoded and checked but not kept: the reader's own `read_file` holds each of them in memory
/// whole, so an archive with a huge earlier entry (a zip bomb) would fill the memory.
pub fn extract(path: &Path, name: &str, max: u64) -> Result<Vec<u8>, ArchiveError> {
    use std::io::Read;
    let (archive, mut file) = open(path)?;
    let index = archive.files.iter().position(|e| e.name.replace('\\', "/") == name && !e.is_directory);
    let index = index.ok_or_else(|| ArchiveError::NoEntry(name.to_string()))?;
    let entry = &archive.files[index];
    if entry.size > max {
        return Err(ArchiveError::TooBig(entry.size));
    }
    let Some(block) = archive.stream_map.file_block_index.get(index).copied().flatten().filter(|_| entry.has_stream) else {
        return Ok(Vec::new());
    };
    if block >= archive.blocks.len() {
        return Err(ArchiveError::Unreadable("damaged archive".into()));
    }
    let password = sevenz_rust2::Password::empty();
    let decoder = sevenz_rust2::BlockDecoder::new_with_memory_limit(1, MEMORY_LIMIT_KB, block, &archive, &password, &mut file);
    let mut data = None;
    let mut too_big = false;
    decoder
        .for_each_entries(&mut |e, r| {
            if !std::ptr::eq(e, entry) {
                // Decoded to nothing and checked; the block's data before the entry is needed.
                return Ok(true);
            }
            let mut out = Vec::with_capacity(e.size.min(4 << 20) as usize);
            r.take(max.saturating_add(1)).read_to_end(&mut out)?;
            too_big = out.len() as u64 > max;
            data = Some(out);
            Ok(false)
        })
        .map_err(unreadable)?;
    if too_big {
        return Err(ArchiveError::TooBig(max.saturating_add(1)));
    }
    data.ok_or_else(|| ArchiveError::Unreadable("the entry's data is missing".into()))
}

#[cfg(test)]
mod robustness;

#[cfg(test)]
#[global_allocator]
static ALLOC: mo_fuzz::CountingAlloc = mo_fuzz::CountingAlloc;

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("mo-archive-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_7z_archive_is_listed_and_opened() {
        let dir = scratch("list");
        let src = dir.join("kaynak");
        std::fs::create_dir_all(src.join("Klasör")).unwrap();
        std::fs::write(src.join("not.txt"), "Merhaba dünya, ğüşıöç").unwrap();
        std::fs::write(src.join("Klasör").join("rapor.csv"), "a;b\n1;2\n").unwrap();
        let archive = dir.join("arsiv.7z");
        sevenz_rust2::compress_to_path(&src, &archive).unwrap();
        assert!(is_7z(&std::fs::read(&archive).unwrap()[..6]));

        let entries = list(&archive).unwrap();
        let files: Vec<&str> = entries.iter().filter(|e| !e.is_dir).map(|e| e.path.as_str()).collect();
        assert!(files.contains(&"not.txt") && files.contains(&"Klasör/rapor.csv"), "{entries:?}");
        assert_eq!(String::from_utf8(extract(&archive, "not.txt", 1 << 20).unwrap()).unwrap(), "Merhaba dünya, ğüşıöç");
        assert_eq!(extract(&archive, "Klasör/rapor.csv", 1 << 20).unwrap(), b"a;b\n1;2\n");
        assert_eq!(extract(&archive, "yok.txt", 1 << 20), Err(ArchiveError::NoEntry("yok.txt".into())));
        assert!(matches!(extract(&archive, "not.txt", 4), Err(ArchiveError::TooBig(_))));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn other_files_are_not_7z() {
        let dir = scratch("other");
        let f = dir.join("x.7z");
        std::fs::write(&f, b"PK\x03\x04 not seven zip").unwrap();
        assert!(!is_7z(b"PK\x03\x04"));
        assert!(matches!(list(&f), Err(ArchiveError::Unreadable(_))));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
