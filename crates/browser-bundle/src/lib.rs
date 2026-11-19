// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Versioned envelope for model-specific browser analyzer recordings.

use std::io::Write;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

const MAGIC: &[u8; 8] = b"QNTBR001";
const CHECKSUM_OFFSET: usize = MAGIC.len() + 4 + 8;
const CHECKSUM_END: usize = CHECKSUM_OFFSET + 8;
const HEADER_LEN: usize = CHECKSUM_END;

/// Current browser bundle envelope version.
pub const FORMAT_VERSION: u32 = 1;

/// Features exposed by a model-specific browser analyzer.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Capabilities {
    pub data_flow: bool,
    pub nvtx: bool,
}

/// Self-describing metadata paired with a schema-specific encoded event payload.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Metadata {
    pub format_version: u32,
    pub model_name: String,
    pub schema_digest: String,
    pub model_provenance: Option<serde_json::Value>,
    pub analyzer_revision: String,
    pub quent_revision: String,
    pub codec: String,
    pub engine_id: Uuid,
    pub context_ids: Vec<Uuid>,
    pub capabilities: Capabilities,
}

/// A validated browser bundle borrowing its encoded model payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bundle<'a> {
    pub metadata: Metadata,
    pub payload: &'a [u8],
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("browser bundle is truncated")]
    Truncated,
    #[error("browser bundle magic is invalid")]
    InvalidMagic,
    #[error("unsupported browser bundle version {0}")]
    UnsupportedVersion(u32),
    #[error("browser bundle length does not fit this platform")]
    LengthOverflow,
    #[error("browser bundle has trailing or missing bytes")]
    InvalidLength,
    #[error("browser bundle checksum does not match")]
    Checksum,
    #[error("browser bundle metadata is invalid: {0}")]
    Metadata(#[from] serde_json::Error),
    #[error("unable to write browser bundle: {0}")]
    Io(#[from] std::io::Error),
}

/// Encode metadata and model-specific payload into a portable browser bundle.
pub fn encode(metadata: &Metadata, payload: &[u8]) -> Result<Vec<u8>, Error> {
    let (header, metadata) = encode_parts(metadata, payload)?;
    let mut bytes = Vec::with_capacity(HEADER_LEN + metadata.len() + payload.len());
    bytes.extend_from_slice(&header);
    bytes.extend_from_slice(&metadata);
    bytes.extend_from_slice(payload);
    Ok(bytes)
}

/// Write metadata and model-specific payload as a portable browser bundle.
pub fn encode_to_writer(
    metadata: &Metadata,
    payload: &[u8],
    mut writer: impl Write,
) -> Result<(), Error> {
    let (header, metadata) = encode_parts(metadata, payload)?;
    writer.write_all(&header)?;
    writer.write_all(&metadata)?;
    writer.write_all(payload)?;
    Ok(())
}

fn encode_parts(metadata: &Metadata, payload: &[u8]) -> Result<([u8; HEADER_LEN], Vec<u8>), Error> {
    if metadata.format_version != FORMAT_VERSION {
        return Err(Error::UnsupportedVersion(metadata.format_version));
    }
    let metadata = serde_json::to_vec(metadata)?;
    let metadata_len = u32::try_from(metadata.len()).map_err(|_| Error::LengthOverflow)?;
    let payload_len = u64::try_from(payload.len()).map_err(|_| Error::LengthOverflow)?;
    let mut header = [0; HEADER_LEN];
    header[..MAGIC.len()].copy_from_slice(MAGIC);
    header[MAGIC.len()..MAGIC.len() + 4].copy_from_slice(&metadata_len.to_be_bytes());
    header[MAGIC.len() + 4..CHECKSUM_OFFSET].copy_from_slice(&payload_len.to_be_bytes());
    let checksum = checksum([&header[..CHECKSUM_OFFSET], &metadata, payload]);
    header[CHECKSUM_OFFSET..CHECKSUM_END].copy_from_slice(&checksum.to_be_bytes());
    Ok((header, metadata))
}

/// Decode and validate a browser bundle without copying its event payload.
pub fn decode(bytes: &[u8]) -> Result<Bundle<'_>, Error> {
    if bytes.len() < HEADER_LEN {
        return Err(Error::Truncated);
    }
    if &bytes[..MAGIC.len()] != MAGIC {
        return Err(Error::InvalidMagic);
    }
    let metadata_len = u32::from_be_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let payload_len = usize::try_from(u64::from_be_bytes(bytes[12..20].try_into().unwrap()))
        .map_err(|_| Error::LengthOverflow)?;
    let expected_checksum =
        u64::from_be_bytes(bytes[CHECKSUM_OFFSET..CHECKSUM_END].try_into().unwrap());
    let metadata_end = HEADER_LEN
        .checked_add(metadata_len)
        .ok_or(Error::LengthOverflow)?;
    let payload_end = metadata_end
        .checked_add(payload_len)
        .ok_or(Error::LengthOverflow)?;
    if payload_end != bytes.len() {
        return Err(Error::InvalidLength);
    }
    let payload = &bytes[metadata_end..payload_end];
    if checksum([&bytes[..CHECKSUM_OFFSET], &bytes[CHECKSUM_END..]]) != expected_checksum {
        return Err(Error::Checksum);
    }
    let metadata: Metadata = serde_json::from_slice(&bytes[HEADER_LEN..metadata_end])?;
    if metadata.format_version != FORMAT_VERSION {
        return Err(Error::UnsupportedVersion(metadata.format_version));
    }
    Ok(Bundle { metadata, payload })
}

// FNV-1a is an integrity check for accidental corruption, not authentication.
fn checksum<'a>(chunks: impl IntoIterator<Item = &'a [u8]>) -> u64 {
    chunks
        .into_iter()
        .flatten()
        .fold(0xcbf29ce484222325, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata() -> Metadata {
        Metadata {
            format_version: FORMAT_VERSION,
            model_name: "test".into(),
            schema_digest: "schema".into(),
            model_provenance: None,
            analyzer_revision: "analyzer".into(),
            quent_revision: "quent".into(),
            codec: "postcard".into(),
            engine_id: Uuid::nil(),
            context_ids: vec![Uuid::nil()],
            capabilities: Capabilities::default(),
        }
    }

    #[test]
    fn round_trip() {
        let bytes = encode(&metadata(), b"events").unwrap();
        let bundle = decode(&bytes).unwrap();
        assert_eq!(bundle.metadata, metadata());
        assert_eq!(bundle.payload, b"events");
    }

    #[test]
    fn writer_matches_in_memory_encoding() {
        let mut bytes = Vec::new();
        encode_to_writer(&metadata(), b"events", &mut bytes).unwrap();
        assert_eq!(bytes, encode(&metadata(), b"events").unwrap());
    }

    #[test]
    fn rejects_corrupt_payload() {
        let mut bytes = encode(&metadata(), b"events").unwrap();
        *bytes.last_mut().unwrap() ^= 1;
        assert!(matches!(decode(&bytes), Err(Error::Checksum)));
    }

    #[test]
    fn rejects_corrupt_metadata() {
        let mut bytes = encode(&metadata(), b"events").unwrap();
        let engine_id = bytes
            .windows(Uuid::nil().hyphenated().to_string().len())
            .position(|window| window == Uuid::nil().hyphenated().to_string().as_bytes())
            .unwrap();
        bytes[engine_id] = b'1';
        assert!(matches!(decode(&bytes), Err(Error::Checksum)));
    }

    #[test]
    fn rejects_trailing_bytes() {
        let mut bytes = encode(&metadata(), b"events").unwrap();
        bytes.push(0);
        assert!(matches!(decode(&bytes), Err(Error::InvalidLength)));
    }
}
