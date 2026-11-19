// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Native construction of portable browser-analyzer bundles.

use std::{collections::HashSet, io::Read, path::Path};

use quent_browser_bundle::{Capabilities, FORMAT_VERSION, Metadata};
use quent_events::{CombinedEventModel, Event, EventModel};
use quent_store::{
    context::ContextSet,
    event::{CombinedEventLoader, filesystem},
};
use uuid::Uuid;

mod error;

pub use error::{Error, Result};

const MAX_ARCHIVE_ENTRIES: usize = 1_000_000;
const MAX_ARCHIVE_BYTES: u64 = 8 * 1024 * 1024 * 1024;

/// Model-specific metadata used to construct a browser bundle.
#[derive(Clone, Debug)]
pub struct BuildOptions<'a> {
    /// Identifies the event model. Stored verbatim in the bundle metadata; analyzers reject
    /// bundles whose `model_name` is not the one they were built for.
    pub model_name: &'a str,
    /// Identifies the serialized event schema. Stored verbatim in the bundle metadata;
    /// analyzers reject bundles whose digest differs from the one for their own build, before
    /// decoding the payload.
    pub schema_digest: &'a str,
    /// Version or commit of the analyzer that produced the bundle. Stored verbatim in the
    /// bundle metadata for provenance; analyzers do not validate it.
    pub analyzer_revision: &'a str,
    pub capabilities: Capabilities,
}

/// Description of a browser bundle written to disk.
#[derive(Clone, Debug)]
pub struct GenerationSummary {
    pub engine_id: Uuid,
    /// Number of context directories included in the bundle: the immediate subdirectories of
    /// the input root (the directory itself, or the ZIP's top level) whose names are canonical,
    /// lowercase, hyphenated UUIDs. Every such directory is loaded, whether or not it contains
    /// events.
    pub context_count: usize,
    /// Total number of events written to the bundle payload, summed across all included
    /// contexts, including the engine events.
    pub event_count: usize,
    /// The `schema_digest` from [`BuildOptions`], as recorded in the bundle metadata.
    pub schema_digest: String,
    pub format_version: u32,
}

/// Convert collector output or a ZIP archive into a browser bundle.
///
/// `engine_id` identifies engine events in the model's combined event stream.
/// Exactly one distinct engine must be present.
pub fn generate_bundle<M>(
    input: &Path,
    output: &Path,
    options: &BuildOptions<'_>,
    engine_id: impl Fn(&Event<<M as CombinedEventModel>::CombinedEvent>) -> Option<Uuid>,
) -> Result<GenerationSummary>
where
    M: EventModel + filesystem::Model,
    <M as CombinedEventModel>::CombinedEvent: serde::Serialize,
{
    let extracted;
    let root = if input.is_dir() {
        input
    } else {
        extracted = tempfile::tempdir()?;
        extract_zip(input, extracted.path())?;
        extracted.path()
    };

    let context_ids = discover_context_ids(root)?;
    if context_ids.is_empty() {
        return Err(Error::NoContexts {
            root: root.to_path_buf(),
        });
    }

    let mut events = Vec::new();
    for context_id in &context_ids {
        events.extend(
            filesystem::Loader::<M>::new(root, ContextSet::one(*context_id))
                .combined_events()?
                .collect::<std::result::Result<Vec<_>, quent_store::Error>>()?,
        );
    }
    events.sort_by_key(|event| (event.timestamp, event.id));

    let engine_ids = events.iter().filter_map(engine_id).collect::<HashSet<_>>();
    if engine_ids.len() != 1 {
        return Err(Error::UnexpectedEngineCount {
            found: engine_ids.len(),
        });
    }
    let engine_id = *engine_ids.iter().next().expect("one engine was validated");
    let model_provenance = read_model_provenance(root, &context_ids)?;
    let event_count = events.len();
    let payload = postcard::to_allocvec(&events)?;
    let quent = quent_events::build_info::quent();
    let bytes = quent_browser_bundle::encode(
        &Metadata {
            format_version: FORMAT_VERSION,
            model_name: options.model_name.to_owned(),
            schema_digest: options.schema_digest.to_owned(),
            model_provenance,
            analyzer_revision: options.analyzer_revision.to_owned(),
            quent_revision: quent.commit.unwrap_or(quent.version),
            codec: "postcard".into(),
            engine_id,
            context_ids: context_ids.clone(),
            capabilities: options.capabilities.clone(),
        },
        &payload,
    )?;
    if let Some(parent) = output.parent().filter(|path| !path.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output, bytes)?;

    Ok(GenerationSummary {
        engine_id,
        context_count: context_ids.len(),
        event_count,
        schema_digest: options.schema_digest.to_owned(),
        format_version: FORMAT_VERSION,
    })
}

fn discover_context_ids(root: &Path) -> Result<Vec<Uuid>> {
    let mut ids = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_type()?.is_dir()
            && let Some(name) = entry.file_name().to_str()
            && let Ok(id) = Uuid::parse_str(name)
            && name == id.hyphenated().to_string()
        {
            ids.push(id);
        }
    }
    ids.sort_unstable();
    Ok(ids)
}

fn read_model_provenance(root: &Path, context_ids: &[Uuid]) -> Result<Option<serde_json::Value>> {
    // The first context that recorded provenance, and its value.
    let mut provenance: Option<(Uuid, serde_json::Value)> = None;
    for &context_id in context_ids {
        let path = root.join(context_id.to_string()).join("model.qmi");
        let value = match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice::<serde_json::Value>(&bytes)
                .map_err(|source| Error::InvalidProvenance { context_id, source })?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        match &provenance {
            Some((first_context_id, first)) if *first != value => {
                return Err(Error::ProvenanceConflict {
                    first_context_id: *first_context_id,
                    conflicting_context_id: context_id,
                });
            }
            Some(_) => {}
            None => provenance = Some((context_id, value)),
        }
    }
    Ok(provenance.map(|(_, value)| value))
}

fn extract_zip(input: &Path, output: &Path) -> Result<()> {
    extract_zip_with_limits(input, output, MAX_ARCHIVE_ENTRIES, MAX_ARCHIVE_BYTES)
}

fn extract_zip_with_limits(
    input: &Path,
    output: &Path,
    max_entries: usize,
    max_bytes: u64,
) -> Result<()> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(input)?)?;
    if archive.len() > max_entries {
        return Err(Error::ArchiveTooManyEntries {
            entries: archive.len(),
            max_entries,
        });
    }

    let mut extracted_bytes = 0_u64;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        let Some(relative) = file.enclosed_name() else {
            return Err(Error::ArchiveUnsafePath {
                name: String::from_utf8_lossy(file.name_raw()).into_owned(),
            });
        };
        reject_special_file(&file, &relative)?;

        // Fail fast on the size declared in the ZIP header. This is attacker-controlled and
        // may understate the real size, so it is only an early check; the copy below enforces
        // the budget against the bytes actually written.
        let remaining = max_bytes - extracted_bytes;
        if file.size() > remaining {
            return Err(Error::ArchiveTooLarge { max_bytes });
        }

        let path = output.join(relative);
        if file.is_dir() {
            std::fs::create_dir_all(path)?;
        } else {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            // Read at most one byte past the budget so an overrun is detectable without ever
            // writing more than `remaining + 1` bytes for this entry.
            let mut limited = (&mut file).take(remaining.saturating_add(1));
            let written = std::io::copy(&mut limited, &mut std::fs::File::create(path)?)?;
            if written > remaining {
                return Err(Error::ArchiveTooLarge { max_bytes });
            }
            extracted_bytes += written;
        }
    }
    Ok(())
}

fn reject_special_file(file: &zip::read::ZipFile<'_, std::fs::File>, path: &Path) -> Result<()> {
    const FILE_TYPE_MASK: u32 = 0o170000;
    const REGULAR_FILE: u32 = 0o100000;
    const DIRECTORY: u32 = 0o040000;

    if let Some(mode) = file.unix_mode() {
        let file_type = mode & FILE_TYPE_MASK;
        if file_type != 0 && file_type != REGULAR_FILE && file_type != DIRECTORY {
            return Err(Error::ArchiveNonRegularEntry {
                path: path.to_path_buf(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use super::*;

    fn build_zip(entries: &[(&str, &[u8])], method: zip::CompressionMethod) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default().compression_method(method);
        for (name, data) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(data).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    fn extract(
        bytes: &[u8],
        max_entries: usize,
        max_bytes: u64,
    ) -> (Result<()>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("in.zip");
        std::fs::write(&zip_path, bytes).unwrap();
        let out = dir.path().join("out");
        std::fs::create_dir(&out).unwrap();
        let result = extract_zip_with_limits(&zip_path, &out, max_entries, max_bytes);
        (result, dir)
    }

    /// Overwrite every little-endian u32 at `field_offset` after each `signature`.
    fn patch_u32(bytes: &mut [u8], signature: &[u8; 4], field_offset: usize, value: u32) {
        for i in 0..bytes.len().saturating_sub(field_offset + 4) {
            if &bytes[i..i + 4] == signature {
                bytes[i + field_offset..i + field_offset + 4].copy_from_slice(&value.to_le_bytes());
            }
        }
    }

    #[test]
    fn extracts_within_limits() {
        let bytes = build_zip(&[("a/b.txt", b"hello")], zip::CompressionMethod::Stored);
        let (result, dir) = extract(&bytes, 10, 5);
        result.unwrap();
        assert_eq!(
            std::fs::read(dir.path().join("out/a/b.txt")).unwrap(),
            b"hello"
        );
    }

    #[test]
    fn rejects_too_many_entries() {
        let bytes = build_zip(
            &[("a", b"1"), ("b", b"2"), ("c", b"3")],
            zip::CompressionMethod::Stored,
        );
        let (result, _dir) = extract(&bytes, 2, 1024);
        assert!(matches!(
            result,
            Err(Error::ArchiveTooManyEntries {
                entries: 3,
                max_entries: 2
            })
        ));
    }

    #[test]
    fn rejects_declared_size_over_budget() {
        let bytes = build_zip(&[("a", &[0u8; 100])], zip::CompressionMethod::Stored);
        let (result, _dir) = extract(&bytes, 10, 99);
        assert!(matches!(result, Err(Error::ArchiveTooLarge { .. })));
    }

    #[test]
    fn rejects_cumulative_size_over_budget() {
        let bytes = build_zip(
            &[("a", &[0u8; 60]), ("b", &[0u8; 60])],
            zip::CompressionMethod::Stored,
        );
        let (result, _dir) = extract(&bytes, 10, 100);
        assert!(matches!(result, Err(Error::ArchiveTooLarge { .. })));
    }

    #[test]
    fn fails_closed_when_header_understates_size() {
        // Highly compressible payload: 1 MiB of zeros deflates to ~1 KiB.
        let payload = vec![0u8; 1024 * 1024];
        let mut bytes = build_zip(&[("a", &payload)], zip::CompressionMethod::Deflated);
        // Lie about the uncompressed size in the local (offset 22) and central (offset 24)
        // headers so the declared-size precheck passes.
        patch_u32(&mut bytes, b"PK\x03\x04", 22, 1);
        patch_u32(&mut bytes, b"PK\x01\x02", 24, 1);

        let (result, dir) = extract(&bytes, 10, 4096);
        // zip >= 9 rejects data that outgrows its declared size (as `Error::Io`); our own cap
        // on bytes written is a second line of defense. Either way extraction must fail and
        // must not write more than the budget plus the single detection byte.
        assert!(
            matches!(result, Err(Error::ArchiveTooLarge { .. } | Error::Io(_))),
            "{result:?}"
        );
        let written = std::fs::metadata(dir.path().join("out/a")).unwrap().len();
        assert!(written <= 4097, "wrote {written} bytes");
    }

    #[test]
    fn rejects_unsafe_archive_path() {
        let bytes = build_zip(&[("../escape", b"x")], zip::CompressionMethod::Stored);
        let (result, _dir) = extract(&bytes, 10, 1024);
        assert!(matches!(result, Err(Error::ArchiveUnsafePath { name }) if name == "../escape"));
    }

    #[test]
    fn rejects_non_archive_input() {
        let (result, _dir) = extract(b"not a zip", 10, 1024);
        assert!(matches!(result, Err(Error::Archive(_))));
    }

    fn write_provenance(root: &Path, context_id: Uuid, contents: &str) {
        let dir = root.join(context_id.to_string());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("model.qmi"), contents).unwrap();
    }

    #[test]
    fn provenance_conflict_names_both_contexts() {
        let root = tempfile::tempdir().unwrap();
        let (first, second) = (Uuid::from_u128(1), Uuid::from_u128(2));
        write_provenance(root.path(), first, r#"{"a":1}"#);
        write_provenance(root.path(), second, r#"{"a":2}"#);

        let error = read_model_provenance(root.path(), &[first, second]).unwrap_err();
        assert!(matches!(
            error,
            Error::ProvenanceConflict {
                first_context_id,
                conflicting_context_id,
            } if first_context_id == first && conflicting_context_id == second
        ));
    }

    #[test]
    fn provenance_ignores_contexts_without_it() {
        let root = tempfile::tempdir().unwrap();
        let (first, second, third) = (Uuid::from_u128(1), Uuid::from_u128(2), Uuid::from_u128(3));
        std::fs::create_dir_all(root.path().join(first.to_string())).unwrap();
        write_provenance(root.path(), second, r#"{"a":1}"#);
        write_provenance(root.path(), third, r#"{"a":1}"#);

        let provenance = read_model_provenance(root.path(), &[first, second, third]).unwrap();
        assert_eq!(provenance, Some(serde_json::json!({"a": 1})));
    }

    #[test]
    fn invalid_provenance_keeps_json_error_as_source() {
        let root = tempfile::tempdir().unwrap();
        let context_id = Uuid::from_u128(1);
        write_provenance(root.path(), context_id, "{not json");

        let error = read_model_provenance(root.path(), &[context_id]).unwrap_err();
        assert!(matches!(
            error,
            Error::InvalidProvenance { context_id: id, .. } if id == context_id
        ));
        let source = std::error::Error::source(&error).expect("source is preserved");
        assert!(source.downcast_ref::<serde_json::Error>().is_some());
    }

    #[test]
    fn ignores_noncanonical_context_directory_names() {
        let root = tempfile::tempdir().unwrap();
        let canonical = Uuid::from_u128(1);
        let noncanonical = Uuid::from_u128(2);
        let uppercase = Uuid::from_u128(0xabcdef);
        std::fs::create_dir(root.path().join(canonical.hyphenated().to_string())).unwrap();
        std::fs::create_dir(root.path().join(noncanonical.simple().to_string())).unwrap();
        std::fs::create_dir(
            root.path()
                .join(uppercase.hyphenated().to_string().to_uppercase()),
        )
        .unwrap();

        assert_eq!(discover_context_ids(root.path()).unwrap(), [canonical]);
    }

    #[test]
    fn discovers_distinct_contexts_in_sorted_order() {
        let root = tempfile::tempdir().unwrap();
        let (a, b) = (Uuid::from_u128(1), Uuid::from_u128(2));
        std::fs::create_dir(root.path().join(b.to_string())).unwrap();
        std::fs::create_dir(root.path().join(a.to_string())).unwrap();
        std::fs::create_dir(root.path().join("not-a-uuid")).unwrap();
        std::fs::write(root.path().join(Uuid::from_u128(3).to_string()), b"file").unwrap();

        assert_eq!(discover_context_ids(root.path()).unwrap(), [a, b]);
    }
}
