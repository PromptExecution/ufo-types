#![cfg(feature = "reqif-archive")]

use std::io::{Cursor, Write};

use serde::Deserialize;
use ufo_types::reqif::archive::ReqIfArchive;
use ufo_types::revision::ArtifactDigest;
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

#[derive(Deserialize)]
struct File {
    path: String,
    bytes: Vec<u8>,
    #[serde(default = "one")]
    repeat: usize,
    symlink: Option<String>,
}

fn one() -> usize {
    1
}

#[derive(Deserialize)]
struct Rewrite {
    from: String,
    to: String,
}

#[derive(Deserialize)]
struct Case {
    name: String,
    #[serde(default)]
    files: Vec<File>,
    error: Option<String>,
    rewrite: Option<Rewrite>,
    raw_length: Option<usize>,
    entry_count: Option<usize>,
    #[serde(default)]
    corrupt: bool,
    declared_size: Option<u32>,
}

fn archive(case: &Case) -> Vec<u8> {
    if let Some(length) = case.raw_length {
        return vec![0; length];
    }
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(if case.corrupt {
        CompressionMethod::Stored
    } else {
        CompressionMethod::Deflated
    });
    for file in &case.files {
        if let Some(target) = &file.symlink {
            writer.add_symlink(&file.path, target, options).unwrap();
        } else if file.path.ends_with('/') {
            writer.add_directory(&file.path, options).unwrap();
        } else {
            writer.start_file(&file.path, options).unwrap();
            writer.write_all(&file.bytes.repeat(file.repeat)).unwrap();
        }
    }
    for index in 0..case.entry_count.unwrap_or(0) {
        writer
            .start_file(format!("{index}.reqif"), options)
            .unwrap();
    }
    let mut bytes = writer.finish().unwrap().into_inner();
    if let Some(size) = case.declared_size {
        let offsets = {
            let mut reader = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
            (0..reader.len())
                .map(|index| reader.by_index(index).unwrap().central_header_start() as usize)
                .collect::<Vec<_>>()
        };
        for offset in offsets {
            // Deliberately dishonest central-directory uncompressed-size field.
            bytes[offset + 24..offset + 28].copy_from_slice(&size.to_le_bytes());
        }
    }
    if let Some(rewrite) = &case.rewrite {
        assert_eq!(rewrite.from.len(), rewrite.to.len());
        let offsets: Vec<_> = bytes
            .windows(rewrite.from.len())
            .enumerate()
            .filter_map(|(offset, value)| (value == rewrite.from.as_bytes()).then_some(offset))
            .collect();
        for offset in offsets {
            bytes[offset..offset + rewrite.from.len()].copy_from_slice(rewrite.to.as_bytes());
        }
    }
    if case.corrupt {
        // Corrupt the stored payload rather than modifying ZIP metadata.
        let position = {
            let mut reader = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
            reader.by_index(0).unwrap().data_start() as usize
        };
        bytes[position] ^= 1;
    }
    bytes
}

#[test]
fn bounded_archive_intake_preserves_bytes_or_returns_typed_failure() {
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("fixtures/reqif_archives.json")).unwrap();
    for case in cases {
        let bytes = archive(&case);
        let result = ReqIfArchive::from_bytes(&bytes);
        if let Some(expected) = &case.error {
            let error = result.unwrap_err().to_string();
            assert!(error.contains(expected), "{}: {error}", case.name);
            continue;
        }
        let archive = result.unwrap();
        assert_eq!(archive.original_bytes, bytes, "{}", case.name);
        assert_eq!(archive.digest(), ArtifactDigest::of(&bytes));
        let files: Vec<_> = case
            .files
            .iter()
            .filter(|file| !file.path.ends_with('/'))
            .collect();
        assert_eq!(archive.entries.len(), files.len());
        for (entry, file) in archive.entries.iter().zip(files) {
            assert_eq!(entry.path.as_str(), file.path);
            assert_eq!(entry.bytes, file.bytes.repeat(file.repeat));
            assert_eq!(entry.digest(), ArtifactDigest::of(&entry.bytes));
        }
        assert_eq!(ReqIfArchive::from_bytes(&bytes).unwrap(), archive);
    }
}
