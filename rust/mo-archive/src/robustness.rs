//! Damaged and hostile 7z archives: listing and opening entries must return (an error is fine)
//! without a panic, within time and memory limits (see `mo-fuzz`).

use std::io::Cursor;

use sevenz_rust2::encoder_options::Lzma2Options;
use sevenz_rust2::{ArchiveEntry, ArchiveWriter, EncoderConfiguration, EncoderMethod, SourceReader};

use super::*;

/// A fresh file path for one test's archives.
fn scratch_file(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("mo-archive-fuzz-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(format!("{tag}.7z"))
}

/// A small, quick LZMA2 setting (a 64 KB dictionary), so tests compress fast.
fn small_lzma2() -> EncoderConfiguration {
    let mut o = Lzma2Options::from_level(1);
    o.set_dictionary_size(64 * 1024);
    o.into()
}

/// A solid archive: `files` packed into one block.
fn solid(files: &[(&str, Vec<u8>)], method: EncoderConfiguration) -> Vec<u8> {
    let mut w = ArchiveWriter::new(Cursor::new(Vec::new())).unwrap();
    w.set_content_methods(vec![method]);
    let entries = files.iter().map(|(n, _)| ArchiveEntry::new_file(n)).collect();
    let readers = files.iter().map(|(_, d)| SourceReader::from(Cursor::new(d.clone()))).collect();
    w.push_archive_entries(entries, readers).unwrap();
    w.finish().unwrap().into_inner()
}

/// A non-solid archive: one block per file, a folder and an empty file.
fn non_solid(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut w = ArchiveWriter::new(Cursor::new(Vec::new())).unwrap();
    w.set_content_methods(vec![small_lzma2()]);
    w.push_archive_entry::<Cursor<Vec<u8>>>(ArchiveEntry::new_directory("Klasör"), None).unwrap();
    w.push_archive_entry(ArchiveEntry::new_file("boş.txt"), Some(Cursor::new(Vec::new()))).unwrap();
    for (n, d) in files {
        w.push_archive_entry(ArchiveEntry::new_file(n), Some(Cursor::new(d.clone()))).unwrap();
    }
    w.finish().unwrap().into_inner()
}

fn seeds() -> Vec<Vec<u8>> {
    let files: Vec<(&str, Vec<u8>)> = vec![
        ("not.txt", "Merhaba dünya, ğüşıöç".as_bytes().to_vec()),
        ("Klasör\\rapor.csv", b"a;b\n1;2\n".to_vec()),
        ("veri.bin", (0..3000u32).map(|i| (i * 7 % 251) as u8).collect()),
    ];
    vec![
        solid(&files, small_lzma2()),
        solid(&files, EncoderConfiguration::new(EncoderMethod::COPY)),
        solid(&files, EncoderConfiguration::new(EncoderMethod::PPMD)),
        non_solid(&files),
    ]
}

#[test]
fn the_seeds_are_read() {
    let path = scratch_file("seeds");
    for seed in seeds() {
        std::fs::write(&path, &seed).unwrap();
        let names: Vec<String> = list(&path).unwrap().into_iter().filter(|e| !e.is_dir).map(|e| e.path).collect();
        assert!(names.contains(&"Klasör/rapor.csv".to_string()), "{names:?}");
        assert_eq!(extract(&path, "Klasör/rapor.csv", 1 << 20).unwrap(), b"a;b\n1;2\n");
        assert_eq!(extract(&path, "veri.bin", 1 << 20).unwrap().len(), 3000);
        assert!(matches!(extract(&path, "veri.bin", 100), Err(ArchiveError::TooBig(_))));
    }
}

#[test]
fn damaged_archives_are_refused_or_read() {
    let seeds = seeds();
    let path = scratch_file("fuzz");
    let tokens: &[&[u8]] = &[b"7z\xbc\xaf\x27\x1c", b"\x00", b"\xff\xff\xff\xff", b"\x01\x04\x06", b"\x17", b"\x0b"];
    mo_fuzz::run(
        "7z",
        mo_fuzz::iterations(1500),
        move |rng, _| {
            let i = rng.below(seeds.len());
            mo_fuzz::mutate(rng, &seeds[i], tokens)
        },
        move |b| {
            std::fs::write(&path, b).unwrap();
            if let Ok(entries) = list(&path) {
                for e in entries.iter().take(8) {
                    let _ = extract(&path, &e.path, 1 << 20);
                }
            }
        },
    );
}

/// A solid archive whose first entry unpacks to far more than the one asked for: opening the
/// small entry must not hold the large one in memory.
#[test]
fn a_large_entry_before_the_one_asked_for_is_not_kept() {
    let big = vec![0u8; 24 << 20];
    let archive = solid(&[("büyük.bin", big), ("küçük.txt", b"tamam".to_vec())], small_lzma2());
    assert!(archive.len() < 1 << 20, "{}", archive.len());
    let path = scratch_file("bomb");
    mo_fuzz::run_with(
        "7z-bomb",
        1,
        mo_fuzz::Limits { memory: 8 << 20, per_input_byte: 4, ..Default::default() },
        move |_, _| archive.clone(),
        move |b| {
            std::fs::write(&path, b).unwrap();
            assert_eq!(extract(&path, "küçük.txt", 1 << 20).unwrap(), b"tamam");
            assert!(matches!(extract(&path, "büyük.bin", 1 << 20), Err(ArchiveError::TooBig(_))));
        },
    );
}
