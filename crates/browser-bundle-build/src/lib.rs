// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Native construction of portable browser-analyzer bundles.

use std::{collections::HashSet, error::Error, io::Read, path::Path};

use quent_browser_bundle::{Capabilities, FORMAT_VERSION, Metadata};
use quent_events::{CombinedEventModel, Event, EventModel};
use quent_store::{
    context::ContextSet,
    event::{CombinedEventLoader, filesystem},
};
use uuid::Uuid;

const MAX_ARCHIVE_ENTRIES: usize = 1_000_000;
const MAX_ARCHIVE_BYTES: u64 = 8 * 1024 * 1024 * 1024;

/// Model-specific metadata used to construct a browser bundle.
#[derive(Clone, Debug)]
pub struct BuildOptions<'a> {
    pub model_name: &'a str,
    pub schema_digest: &'a str,
    pub analyzer_revision: &'a str,
    pub capabilities: Capabilities,
}

/// Description of a browser bundle written to disk.
#[derive(Clone, Debug)]
pub struct GenerationSummary {
    pub engine_id: Uuid,
    pub context_count: usize,
    pub event_count: usize,
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
) -> Result<GenerationSummary, Box<dyn Error>>
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
        return Err(format!("no UUID context directories found under {}", root.display()).into());
    }

    let mut events = Vec::new();
    for context_id in &context_ids {
        events.extend(
            filesystem::Loader::<M>::new(root, ContextSet::one(*context_id))
                .combined_events()?
                .collect::<Result<Vec<_>, _>>()?,
        );
    }
    events.sort_by_key(|event| (event.timestamp, event.id));

    let engine_ids = events.iter().filter_map(engine_id).collect::<HashSet<_>>();
    if engine_ids.len() != 1 {
        return Err(format!(
            "expected exactly one engine in browser bundle, found {}",
            engine_ids.len()
        )
        .into());
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

fn discover_context_ids(root: &Path) -> Result<Vec<Uuid>, std::io::Error> {
    let mut ids = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_type()?.is_dir()
            && let Some(name) = entry.file_name().to_str()
            && let Ok(id) = Uuid::parse_str(name)
        {
            ids.push(id);
        }
    }
    ids.sort_unstable();
    Ok(ids)
}

fn read_model_provenance(
    root: &Path,
    context_ids: &[Uuid],
) -> Result<Option<serde_json::Value>, Box<dyn Error>> {
    let mut provenance = None;
    for context_id in context_ids {
        let path = root.join(context_id.to_string()).join("model.qmi");
        let value = match std::fs::read(path) {
            Ok(bytes) => Some(serde_json::from_slice(&bytes)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        if let (Some(first), Some(current)) = (&provenance, &value)
            && first != current
        {
            return Err("browser bundle contexts have inconsistent model provenance".into());
        }
        if provenance.is_none() {
            provenance = value;
        }
    }
    Ok(provenance)
}

fn extract_zip(input: &Path, output: &Path) -> Result<(), Box<dyn Error>> {
    extract_zip_with_limits(input, output, MAX_ARCHIVE_ENTRIES, MAX_ARCHIVE_BYTES)
}

fn extract_zip_with_limits(
    input: &Path,
    output: &Path,
    max_entries: usize,
    max_bytes: u64,
) -> Result<(), Box<dyn Error>> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(input)?)?;
    if archive.len() > max_entries {
        return Err(format!(
            "ZIP contains too many entries: {} exceeds {max_entries}",
            archive.len()
        )
        .into());
    }

    let mut extracted_bytes = 0_u64;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        let Some(relative) = file.enclosed_name() else {
            return Err(format!("ZIP contains unsafe path: {}", file.name()).into());
        };
        reject_special_file(&file, &relative)?;

        // Fail fast on the size declared in the ZIP header. This is attacker-controlled and
        // may understate the real size, so it is only an early check; the copy below enforces
        // the budget against the bytes actually written.
        let remaining = max_bytes - extracted_bytes;
        if file.size() > remaining {
            return Err(archive_too_large(max_bytes).into());
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
                return Err(archive_too_large(max_bytes).into());
            }
            extracted_bytes += written;
        }
    }
    Ok(())
}

fn archive_too_large(max_bytes: u64) -> String {
    format!("ZIP expands beyond the {max_bytes}-byte safety limit")
}

fn reject_special_file(
    file: &zip::read::ZipFile<'_, std::fs::File>,
    path: &Path,
) -> Result<(), Box<dyn Error>> {
    const FILE_TYPE_MASK: u32 = 0o170000;
    const REGULAR_FILE: u32 = 0o100000;
    const DIRECTORY: u32 = 0o040000;

    if let Some(mode) = file.unix_mode() {
        let file_type = mode & FILE_TYPE_MASK;
        if file_type != 0 && file_type != REGULAR_FILE && file_type != DIRECTORY {
            return Err(format!("ZIP contains non-regular entry: {}", path.display()).into());
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
    ) -> (Result<(), String>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("in.zip");
        std::fs::write(&zip_path, bytes).unwrap();
        let out = dir.path().join("out");
        std::fs::create_dir(&out).unwrap();
        let result = extract_zip_with_limits(&zip_path, &out, max_entries, max_bytes)
            .map_err(|e| e.to_string());
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
        assert!(result.unwrap_err().contains("too many entries"));
    }

    #[test]
    fn rejects_declared_size_over_budget() {
        let bytes = build_zip(&[("a", &[0u8; 100])], zip::CompressionMethod::Stored);
        let (result, _dir) = extract(&bytes, 10, 99);
        assert!(result.unwrap_err().contains("safety limit"));
    }

    #[test]
    fn rejects_cumulative_size_over_budget() {
        let bytes = build_zip(
            &[("a", &[0u8; 60]), ("b", &[0u8; 60])],
            zip::CompressionMethod::Stored,
        );
        let (result, _dir) = extract(&bytes, 10, 100);
        assert!(result.unwrap_err().contains("safety limit"));
    }

    #[test]
    fn enforces_budget_when_header_understates_size() {
        // Highly compressible payload: 1 MiB of zeros deflates to ~1 KiB.
        let payload = vec![0u8; 1024 * 1024];
        let mut bytes = build_zip(&[("a", &payload)], zip::CompressionMethod::Deflated);
        // Lie about the uncompressed size in the local (offset 22) and central (offset 24)
        // headers so the declared-size precheck passes.
        patch_u32(&mut bytes, b"PK\x03\x04", 22, 1);
        patch_u32(&mut bytes, b"PK\x01\x02", 24, 1);

        let (result, dir) = extract(&bytes, 10, 4096);
        assert!(result.unwrap_err().contains("safety limit"));
        // Never write more than the budget plus the single detection byte.
        let written = std::fs::metadata(dir.path().join("out/a")).unwrap().len();
        assert!(written <= 4097, "wrote {written} bytes");
    }
}
