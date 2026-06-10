// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Basic traits for exporter / importer implementations

use quent_events::Event;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExporterError {
    #[error("i/o error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("flush error: {0}")]
    Flush(String),
    #[error("serde error: {0}")]
    Serde(String),
    #[error("collector error: {0}")]
    Collector(String),
}

#[derive(Error, Debug)]
pub enum ImporterError {
    #[error("i/o error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type ExporterResult<T> = std::result::Result<T, ExporterError>;
pub type ImporterResult<T> = std::result::Result<T, ImporterError>;

/// Magic prefix at the start of a binary (msgpack/postcard) artifact carrying an
/// `ArtifactInfo` header. Followed by a one-byte format tag and a single
/// length-prefixed header record. Legacy headerless files lack this prefix.
pub const ARTIFACT_MAGIC: &[u8; 4] = b"QNT1";
/// Format tag for msgpack-encoded header payloads.
pub const FORMAT_MSGPACK: u8 = 0x01;
/// Format tag for postcard-encoded header payloads.
pub const FORMAT_POSTCARD: u8 = 0x02;

/// The leading bytes of a binary artifact file, classified by [`read_preamble`].
pub enum Preamble {
    /// A header is present; the payload is the encoded `ArtifactInfo` bytes (the
    /// caller decodes them with the format's own decoder).
    Header(Vec<u8>),
    /// No header (legacy file); these four bytes are the first event record's
    /// length prefix and must be fed back into event iteration.
    Legacy([u8; 4]),
    /// Empty file.
    Empty,
}

/// Read the leading preamble of a binary artifact. Reads only the header (never
/// an event), so it is safe to call before iterating events or to peek a header.
pub fn read_preamble<R: std::io::Read>(reader: &mut R) -> ImporterResult<Preamble> {
    let mut magic = [0u8; 4];
    match reader.read_exact(&mut magic) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(Preamble::Empty),
        Err(e) => return Err(e.into()),
    }
    if &magic != ARTIFACT_MAGIC {
        return Ok(Preamble::Legacy(magic));
    }
    let mut format = [0u8; 1];
    reader.read_exact(&mut format)?;
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf)?;
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload)?;
    Ok(Preamble::Header(payload))
}

#[async_trait::async_trait]
pub trait Exporter<T>: Send + Sync
where
    T: Serialize + Send,
{
    async fn push(&self, event: Event<T>) -> ExporterResult<()>;
    async fn force_flush(&self) -> ExporterResult<()>;
}

pub trait Importer<T>: Iterator<Item = Event<T>>
where
    T: for<'de> Deserialize<'de>,
{
}
