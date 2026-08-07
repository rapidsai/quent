// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

pub mod exporter;
pub mod importer;

/// Serialization format for the filesystem exporter and importer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    #[cfg(feature = "ndjson")]
    Ndjson,
    #[cfg(feature = "msgpack")]
    Msgpack,
    #[cfg(feature = "postcard")]
    Postcard,
}

impl TryFrom<&str> for Format {
    type Error = String;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value.to_ascii_lowercase().as_str() {
            #[cfg(feature = "ndjson")]
            "ndjson" => Self::Ndjson,
            #[cfg(feature = "msgpack")]
            "msgpack" => Self::Msgpack,
            #[cfg(feature = "postcard")]
            "postcard" => Self::Postcard,
            _ => return Err(format!("invalid filesystem format '{value}'")),
        })
    }
}

impl Format {
    /// Detect the format of a context directory from the first recognized
    /// `*.<ext>` event stream in any of its per-entity subdirectories. Returns
    /// `None` if no readable stream with a known extension is present.
    pub fn detect(context_dir: &std::path::Path) -> Option<Self> {
        for entry in std::fs::read_dir(context_dir).ok()?.flatten() {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let Ok(files) = std::fs::read_dir(entry.path()) else {
                continue;
            };
            for file in files.flatten() {
                if let Some(format) = std::path::Path::new(&file.file_name())
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .and_then(|ext| Self::try_from(ext).ok())
                {
                    return Some(format);
                }
            }
        }
        None
    }
}
