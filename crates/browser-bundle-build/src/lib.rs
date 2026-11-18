// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Native construction of portable browser-analyzer bundles.

use std::{collections::HashSet, error::Error, path::Path};

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
    let mut archive = zip::ZipArchive::new(std::fs::File::open(input)?)?;
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(format!(
            "ZIP contains too many entries: {} exceeds {MAX_ARCHIVE_ENTRIES}",
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
        extracted_bytes = extracted_bytes
            .checked_add(file.size())
            .ok_or("ZIP extracted size overflow")?;
        if extracted_bytes > MAX_ARCHIVE_BYTES {
            return Err(
                format!("ZIP expands beyond the {MAX_ARCHIVE_BYTES}-byte safety limit").into(),
            );
        }

        let path = output.join(relative);
        if file.is_dir() {
            std::fs::create_dir_all(path)?;
        } else {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::io::copy(&mut file, &mut std::fs::File::create(path)?)?;
        }
    }
    Ok(())
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
