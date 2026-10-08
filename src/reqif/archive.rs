//! Bounded ReqIFz transport, without extraction or implicit semantic assertions.
//!
//! Original archive bytes and central-directory entry order survive intake. Nested
//! archives are opaque attachments: this layer never recursively decompresses them.
//! XML/schema validation and typed projection must run before model acceptance.

use std::collections::BTreeSet;
use std::io::{Cursor, Read};

use crate::revision::{ArtifactDigest, ArtifactPath};

pub const MAX_ARCHIVE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_ENTRY_BYTES: u64 = 16 * 1024 * 1024;
pub const MAX_EXPANDED_BYTES: u64 = 32 * 1024 * 1024;
pub const MAX_ARCHIVE_ENTRIES: usize = 1024;
pub const MAX_ENTRY_PATH_BYTES: usize = 1024;

/// Transport failures are distinct from a ReqIF semantic/fidelity result.
#[derive(Debug, thiserror::Error)]
pub enum ReqIfArchiveError {
    #[error("archive limit exceeded: {0}")]
    Limit(&'static str),
    #[error("unsupported or ambiguous archive path: {0}")]
    Path(String),
    #[error("unsupported archive entry: {0}")]
    Unsupported(String),
    #[error("archive has no .reqif document")]
    MissingDocument,
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
    #[error(transparent)]
    Metadata(#[from] rawzip::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReqIfArchiveEntry {
    pub path: ArtifactPath,
    pub bytes: Vec<u8>,
    /// Filename classification only; this does not claim valid ReqIF XML.
    pub is_document: bool,
}

impl ReqIfArchiveEntry {
    pub fn digest(&self) -> ArtifactDigest {
        ArtifactDigest::of(&self.bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReqIfArchive {
    /// Includes directory entries, comments, compression and original ZIP metadata.
    pub original_bytes: Vec<u8>,
    /// Non-directory entries in their original central-directory order.
    pub entries: Vec<ReqIfArchiveEntry>,
}

impl ReqIfArchive {
    pub fn digest(&self) -> ArtifactDigest {
        ArtifactDigest::of(&self.original_bytes)
    }

    /// Read by index so duplicate ZIP names cannot alias an earlier entry.
    /// No host paths are opened, and returned bytes have passed ZIP CRC checks.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ReqIfArchiveError> {
        if bytes.len() > MAX_ARCHIVE_BYTES {
            return Err(ReqIfArchiveError::Limit("compressed bytes"));
        }
        // zip 2.x indexes by name and silently collapses duplicates. Enumerate
        // raw central records first, before its allocating/name-indexing parser.
        let metadata = rawzip::ZipArchive::from_slice(bytes)?;
        if metadata.entries_hint() > MAX_ARCHIVE_ENTRIES as u64 {
            return Err(ReqIfArchiveError::Limit("entry count"));
        }
        let mut records = metadata.entries();
        let mut raw_names = Vec::new();
        let mut unique_names = BTreeSet::new();
        while let Some(record) = records.next_entry()? {
            if raw_names.len() == MAX_ARCHIVE_ENTRIES {
                return Err(ReqIfArchiveError::Limit("entry count"));
            }
            let name = record.file_path().as_bytes();
            if name.len() > MAX_ENTRY_PATH_BYTES + 1 {
                return Err(ReqIfArchiveError::Limit("path bytes"));
            }
            if !unique_names.insert(name.to_vec()) {
                return Err(ReqIfArchiveError::Path(
                    String::from_utf8_lossy(name).into(),
                ));
            }
            raw_names.push(name.to_vec());
        }
        if raw_names.len() as u64 != metadata.entries_hint() {
            return Err(ReqIfArchiveError::Unsupported(
                "central-directory count".into(),
            ));
        }
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes))?;
        if zip.len() != raw_names.len() {
            return Err(ReqIfArchiveError::Unsupported(
                "central-directory identity".into(),
            ));
        }
        let mut paths = BTreeSet::new();
        let mut file_paths = BTreeSet::new();
        let mut entries = Vec::new();
        let mut total = 0_u64;
        for (index, raw_name) in raw_names.iter().enumerate() {
            let mut entry = zip.by_index(index)?;
            if entry.name_raw() != raw_name {
                return Err(ReqIfArchiveError::Unsupported(
                    "central-directory order".into(),
                ));
            }
            let raw_name = std::str::from_utf8(entry.name_raw())
                .map_err(|_| ReqIfArchiveError::Unsupported("non-UTF-8 path".into()))?;
            let is_dir = entry.is_dir();
            let name = if is_dir {
                raw_name.strip_suffix('/').unwrap_or(raw_name)
            } else {
                raw_name
            };
            if name.len() > MAX_ENTRY_PATH_BYTES {
                return Err(ReqIfArchiveError::Limit("path bytes"));
            }
            let path = ArtifactPath::new(name).map_err(|_| ReqIfArchiveError::Path(name.into()))?;
            if !paths.insert(name.to_owned()) {
                return Err(ReqIfArchiveError::Path(name.into()));
            }
            // Symlinks and special files are not portable source artifacts.
            if entry.unix_mode().is_some_and(|mode| {
                let kind = mode & 0o170000;
                kind != 0 && kind != if is_dir { 0o040000 } else { 0o100000 }
            }) {
                return Err(ReqIfArchiveError::Unsupported(name.into()));
            }
            let name = name.to_owned();
            if is_dir {
                if entry.size() != 0 {
                    return Err(ReqIfArchiveError::Unsupported(name));
                }
                continue;
            }
            file_paths.insert(name.clone());
            if entry.size() > MAX_ENTRY_BYTES {
                return Err(ReqIfArchiveError::Limit("entry bytes"));
            }
            let remaining = MAX_EXPANDED_BYTES - total;
            if entry.size() > remaining {
                return Err(ReqIfArchiveError::Limit("expanded bytes"));
            }
            let bound = MAX_ENTRY_BYTES.min(remaining);
            // Do not reserve attacker-declared size; cap actual decompressed reads too.
            let mut content = Vec::new();
            entry.by_ref().take(bound + 1).read_to_end(&mut content)?;
            if content.len() as u64 > bound {
                return Err(ReqIfArchiveError::Limit("expanded bytes"));
            }
            if content.len() as u64 != entry.size() {
                return Err(ReqIfArchiveError::Unsupported(name));
            }
            total += content.len() as u64;
            entries.push(ReqIfArchiveEntry {
                path,
                bytes: content,
                is_document: name.to_ascii_lowercase().ends_with(".reqif"),
            });
        }
        // Reject a file also used as a directory, regardless of entry ordering.
        for path in &paths {
            for (offset, _) in path.match_indices('/') {
                if file_paths.contains(&path[..offset]) {
                    return Err(ReqIfArchiveError::Path(path.clone()));
                }
            }
        }
        if !entries.iter().any(|entry| entry.is_document) {
            return Err(ReqIfArchiveError::MissingDocument);
        }
        Ok(Self {
            original_bytes: bytes.to_vec(),
            entries,
        })
    }
}
