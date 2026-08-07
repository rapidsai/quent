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
    /// Detect the common format of a context's event streams. Returns `None` if
    /// no recognized stream is present or streams use different formats.
    pub fn detect(context_dir: &std::path::Path) -> Option<Self> {
        let mut detected = None;
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
                    match detected {
                        Some(existing) if existing != format => return None,
                        None => detected = Some(format),
                        _ => {}
                    }
                }
            }
        }
        detected
    }
}

#[cfg(all(test, feature = "ndjson"))]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use uuid::Uuid;

    fn context_with_streams(streams: &[(&str, &str)]) -> PathBuf {
        let context = std::env::temp_dir().join(format!("quent-format-{}", Uuid::now_v7()));
        for &(entity, extension) in streams {
            let stream_dir = context.join(entity);
            std::fs::create_dir_all(&stream_dir).unwrap();
            std::fs::write(stream_dir.join(format!("events.{extension}")), []).unwrap();
        }
        context
    }

    #[test]
    fn detects_one_context_wide_format() {
        let context =
            context_with_streams(&[("EngineEvent", "ndjson"), ("NvtxEventEntity", "ndjson")]);
        let detected = Format::detect(&context);
        std::fs::remove_dir_all(context).unwrap();

        assert_eq!(detected, Some(Format::Ndjson));
    }

    #[test]
    #[cfg(feature = "msgpack")]
    fn rejects_mixed_context_formats() {
        let context =
            context_with_streams(&[("EngineEvent", "ndjson"), ("NvtxEventEntity", "msgpack")]);
        let detected = Format::detect(&context);
        std::fs::remove_dir_all(context).unwrap();

        assert_eq!(detected, None);
    }
}
