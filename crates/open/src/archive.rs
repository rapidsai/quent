// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Extract telemetry archives (tar family or zip) into a scratch directory, with
//! guards against oversized/malicious archives. Shared by the `local` loader (to
//! open archive inputs) and the `db` loader (to open a run's downloaded assets).

use std::io::Read;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::error::{OpenError, Result};

/// Filename suffixes recognized as archives (tar family or zip).
const ARCHIVE_SUFFIXES: &[&str] = &[".tar", ".tar.gz", ".tgz", ".tar.zst", ".tzst", ".zip"];

/// Run-wide cap on bytes extracted across all archives (their staged dirs all live
/// until the viewer exits), so aggregate extraction can't exhaust the disk.
pub(crate) const MAX_RUN_UNPACKED_BYTES: u64 = 32 * 1024 * 1024 * 1024;

/// Per-archive cap on extracted bytes. Bounds a single archive — and so the
/// in-memory read of any one GNU long-name/PAX extension record, which `tar`
/// buffers before we can inspect it — independently of the run-wide budget.
///
/// Residual: `tar` exposes no hook to cap an extension record at the KB scale a
/// legit path needs, so a hostile long-name is bounded only to this per-archive
/// cap, not smaller. Acceptable given the trusted source; a raw tar-header parser
/// would be needed to tighten it further.
const MAX_ARCHIVE_UNPACKED_BYTES: u64 = 8 * 1024 * 1024 * 1024;

/// Run-wide cap on the number of extracted entries, so an archive of millions of
/// tiny files/dirs can't exhaust inodes under the byte budgets.
pub(crate) const MAX_ENTRIES: u64 = 1_000_000;

/// Whether `filename` looks like a supported archive (tar family or zip).
pub(crate) fn is_archive(filename: &str) -> bool {
    let lower = filename.to_ascii_lowercase();
    ARCHIVE_SUFFIXES
        .iter()
        .any(|suffix| lower.ends_with(suffix))
}

/// Collect archive files at or under `path`: `path` itself when it is an archive
/// file, plus any archive files found while walking it when it is a directory.
pub(crate) fn find_archives(path: &Path) -> Vec<PathBuf> {
    WalkDir::new(path)
        .into_iter()
        .flatten()
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.file_name().to_str().is_some_and(is_archive))
        .map(walkdir::DirEntry::into_path)
        .collect()
}

/// Extract an archive (tar, optionally gzip/zstd compressed, or zip) at `src` into
/// `dest`. Entries that escape `dest` or aren't plain files/dirs (symlinks,
/// hardlinks, devices) are rejected; extracted bytes are capped per-archive and
/// against the run-wide `unpacked_remaining`, and the entry count against
/// `entries_remaining`.
pub(crate) fn extract_archive(
    dest: &Path,
    filename: &str,
    src: &Path,
    unpacked_remaining: &mut u64,
    entries_remaining: &mut u64,
) -> Result<()> {
    let lower = filename.to_ascii_lowercase();
    if lower.ends_with(".zip") {
        return unpack_zip(src, dest, unpacked_remaining, entries_remaining);
    }
    let reader = std::io::BufReader::new(std::fs::File::open(src)?);
    if lower.ends_with(".tar.zst") || lower.ends_with(".tzst") {
        unpack_tar(
            zstd::stream::read::Decoder::new(reader)?,
            dest,
            unpacked_remaining,
            entries_remaining,
        )
    } else if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
        unpack_tar(
            flate2::read::GzDecoder::new(reader),
            dest,
            unpacked_remaining,
            entries_remaining,
        )
    } else {
        unpack_tar(reader, dest, unpacked_remaining, entries_remaining)
    }
}

/// A reader that errors once its (per-archive) `archive_remaining` quota or the
/// shared run-wide `run_remaining` budget is exhausted, so an oversized stream
/// (e.g. a decompression bomb, or a giant GNU long-name/PAX record `tar` buffers
/// internally) fails loudly instead of being silently truncated at a
/// [`Read::take`] EOF that `tar` would treat as success. Both counters are
/// borrowed so they persist across an archive's entries.
struct LimitReader<'a, R> {
    inner: R,
    archive_remaining: &'a mut u64,
    run_remaining: &'a mut u64,
}

impl<R: Read> Read for LimitReader<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let allowed = (*self.archive_remaining).min(*self.run_remaining);
        if allowed == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "archive exceeds the unpack size limit",
            ));
        }
        let cap = buf.len().min(allowed as usize);
        let read = self.inner.read(&mut buf[..cap])?;
        *self.archive_remaining -= read as u64;
        *self.run_remaining -= read as u64;
        Ok(read)
    }
}

/// Draw one entry against the run-wide entry budget, erroring when it's exhausted.
fn take_entry(entries_remaining: &mut u64) -> Result<()> {
    if *entries_remaining == 0 {
        return Err(OpenError::BadArtifactLayout {
            detail: format!("archives exceed the {MAX_ENTRIES}-entry limit"),
        });
    }
    *entries_remaining -= 1;
    Ok(())
}

/// Force an extracted scratch directory owner-writable/traversable, so a read-only
/// archive mode (e.g. `0555`) can't block extracting its children or removing the
/// tree on cleanup.
#[cfg(unix)]
fn ensure_dir_writable(dir: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(metadata) = std::fs::metadata(dir) {
        let mut perms = metadata.permissions();
        perms.set_mode(perms.mode() | 0o700);
        let _ = std::fs::set_permissions(dir, perms);
    }
}
#[cfg(not(unix))]
fn ensure_dir_writable(_dir: &Path) {}

/// Unpack a tar archive under `root`. Rejects entries that escape `root` (paths
/// with `..` or an absolute/root prefix — [`tar`] skips those, returning `false`)
/// and entries that aren't plain files or directories (symlinks/hardlinks/devices,
/// which could redirect writes outside `root`). Extracted bytes are capped
/// per-archive and against the shared `unpacked_remaining`; the entry count is
/// capped against `entries_remaining`. Extracted directories are forced
/// owner-writable (see [`ensure_dir_writable`]).
fn unpack_tar<R: Read>(
    reader: R,
    root: &Path,
    unpacked_remaining: &mut u64,
    entries_remaining: &mut u64,
) -> Result<()> {
    let mut archive_remaining = MAX_ARCHIVE_UNPACKED_BYTES;
    let reader = LimitReader {
        inner: reader,
        archive_remaining: &mut archive_remaining,
        run_remaining: unpacked_remaining,
    };
    let mut archive = tar::Archive::new(reader);
    archive.set_preserve_mtime(false);
    for entry in archive.entries()? {
        take_entry(entries_remaining)?;
        let mut entry = entry?;
        let kind = entry.header().entry_type();
        let relative = entry.path().map(|p| p.into_owned()).unwrap_or_default();
        if !(kind.is_file() || kind.is_dir()) {
            return Err(OpenError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "refusing non-regular archive entry ({kind:?}): {}",
                    relative.display()
                ),
            )));
        }
        if !entry.unpack_in(root)? {
            return Err(OpenError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "refusing archive entry that escapes the scratch dir: {}",
                    relative.display()
                ),
            )));
        }
        // Directories precede their children in a tar; make this one writable
        // before those children arrive.
        if kind.is_dir() {
            ensure_dir_writable(&root.join(&relative));
        }
    }
    Ok(())
}

/// Unpack a zip archive at `src` under `root`, with the same guards as
/// [`unpack_tar`]: reject entries that escape `root` or aren't plain files/dirs
/// (e.g. unix symlinks), cap extracted bytes per-archive and against
/// `unpacked_remaining`, cap the entry count against `entries_remaining`, and force
/// extracted directories owner-writable.
fn unpack_zip(
    src: &Path,
    root: &Path,
    unpacked_remaining: &mut u64,
    entries_remaining: &mut u64,
) -> Result<()> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(src)?).map_err(zip_error)?;
    let mut archive_remaining = MAX_ARCHIVE_UNPACKED_BYTES;
    for index in 0..archive.len() {
        take_entry(entries_remaining)?;
        let mut entry = archive.by_index(index).map_err(zip_error)?;
        // A path that would escape `root` (`..`, absolute) has no enclosed name.
        let Some(relative) = entry.enclosed_name() else {
            return Err(OpenError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "refusing zip entry that escapes the scratch dir: {}",
                    entry.name()
                ),
            )));
        };
        // Reject anything that isn't a plain file or directory (e.g. unix symlinks
        // stored via the mode bits), which could redirect writes outside `root`.
        if let Some(mode) = entry.unix_mode() {
            let format = mode & 0o170000;
            if format != 0 && format != 0o040000 && format != 0o100000 {
                return Err(OpenError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "refusing non-regular zip entry ({mode:o}): {}",
                        relative.display()
                    ),
                )));
            }
        }
        let out = root.join(&relative);
        if entry.is_dir() {
            std::fs::create_dir_all(&out)?;
            ensure_dir_writable(&out);
        } else {
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut writer = std::fs::File::create(&out)?;
            let mut reader = LimitReader {
                inner: &mut entry,
                archive_remaining: &mut archive_remaining,
                run_remaining: unpacked_remaining,
            };
            std::io::copy(&mut reader, &mut writer)?;
        }
    }
    Ok(())
}

/// Map a `zip` error to an [`OpenError`] (bad/unsupported archive contents).
fn zip_error(error: zip::result::ZipError) -> OpenError {
    OpenError::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("invalid zip archive: {error}"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::discover_contexts;
    use quent_build_info::SIDECAR_FILE_NAME;
    use std::io::Write;

    fn add_entry(builder: &mut tar::Builder<Vec<u8>>, path: &str, data: &[u8]) {
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder.append_data(&mut header, path, data).unwrap();
    }

    /// Build an in-memory tar of a `<uuid>/model.qmi` + `<entity>/x.ndjson` context.
    fn context_tar() -> Vec<u8> {
        let mut builder = tar::Builder::new(Vec::new());
        add_entry(&mut builder, "ctx-uuid/model.qmi", b"{}");
        add_entry(&mut builder, "ctx-uuid/engine/x.ndjson", b"");
        builder.into_inner().unwrap()
    }

    /// Build an in-memory zip nesting a context under `telemetry/`, like the real
    /// run-assets zip.
    fn context_zip() -> Vec<u8> {
        let mut buf = Vec::new();
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let opts = zip::write::SimpleFileOptions::default();
        zip.start_file("telemetry/ctx-uuid/model.qmi", opts)
            .unwrap();
        zip.write_all(b"{}").unwrap();
        zip.start_file("telemetry/ctx-uuid/engine/x.ndjson", opts)
            .unwrap();
        zip.finish().unwrap();
        buf
    }

    /// Write archive bytes to a temp file (`extract_archive` reads from a path).
    fn archive_file(bytes: &[u8]) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(bytes).unwrap();
        file
    }

    /// Generous run budgets for extraction tests that don't exercise the caps.
    fn budgets() -> (u64, u64) {
        (MAX_RUN_UNPACKED_BYTES, MAX_ENTRIES)
    }

    /// Hand-build a one-entry ustar archive with an arbitrary `name`/`typeflag`
    /// (the safe `tar::Builder` API rejects `..` and normalizes links, so we can't
    /// use it to forge malicious entries).
    fn raw_tar(name: &str, link: &str, typeflag: u8, data: &[u8]) -> Vec<u8> {
        let mut header = [0u8; 512];
        header[..name.len()].copy_from_slice(name.as_bytes());
        header[100..108].copy_from_slice(b"0000644\0"); // mode
        header[108..116].copy_from_slice(b"0000000\0"); // uid
        header[116..124].copy_from_slice(b"0000000\0"); // gid
        header[124..136].copy_from_slice(format!("{:011o}\0", data.len()).as_bytes()); // size
        header[136..148].copy_from_slice(b"00000000000\0"); // mtime
        header[156] = typeflag;
        header[157..157 + link.len()].copy_from_slice(link.as_bytes()); // linkname
        header[257..263].copy_from_slice(b"ustar\0");
        header[263..265].copy_from_slice(b"00");
        header[148..156].copy_from_slice(b"        "); // checksum field spaces before summing
        let checksum: u32 = header.iter().map(|&b| b as u32).sum();
        header[148..156].copy_from_slice(format!("{checksum:06o}\0 ").as_bytes());

        let mut out = header.to_vec();
        out.extend_from_slice(data);
        out.resize(out.len().div_ceil(512) * 512, 0); // pad final block
        out.extend(std::iter::repeat_n(0, 1024)); // two zero blocks terminate the archive
        out
    }

    #[test]
    fn is_archive_matches_supported_suffixes() {
        assert!(is_archive("run.tar"));
        assert!(is_archive("run.tar.zst"));
        assert!(is_archive("RUN.TAR.GZ"));
        assert!(is_archive("run_assets.zip"));
        assert!(!is_archive("traces.parquet"));
        assert!(!is_archive("report.nsys-rep"));
    }

    #[test]
    fn extract_tar_then_discover_finds_context() {
        let dest = tempfile::tempdir().unwrap();
        let archive = archive_file(&context_tar());
        let (mut bytes, mut entries) = budgets();
        extract_archive(
            dest.path(),
            "run.tar",
            archive.path(),
            &mut bytes,
            &mut entries,
        )
        .unwrap();

        let found = discover_contexts(&[dest.path().to_path_buf()]).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].file_name().unwrap(), "ctx-uuid");
        assert!(found[0].join(SIDECAR_FILE_NAME).is_file());
    }

    #[test]
    fn extract_zip_then_discover_finds_nested_context() {
        let dest = tempfile::tempdir().unwrap();
        let archive = archive_file(&context_zip());
        let (mut bytes, mut entries) = budgets();
        extract_archive(
            dest.path(),
            "run.zip",
            archive.path(),
            &mut bytes,
            &mut entries,
        )
        .unwrap();

        let found = discover_contexts(&[dest.path().to_path_buf()]).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].file_name().unwrap(), "ctx-uuid");
        assert!(found[0].join(SIDECAR_FILE_NAME).is_file());
    }

    #[test]
    fn extract_tolerates_readonly_dir_headers() {
        // A read-only dir header (0555) before its child must not block extracting
        // the child; scratch dirs are forced owner-writable.
        let mut builder = tar::Builder::new(Vec::new());
        let mut dir = tar::Header::new_gnu();
        dir.set_entry_type(tar::EntryType::Directory);
        dir.set_size(0);
        dir.set_mode(0o555);
        dir.set_cksum();
        builder
            .append_data(&mut dir, "ro/", std::io::empty())
            .unwrap();
        add_entry(&mut builder, "ro/child.txt", b"hi");
        let archive = archive_file(&builder.into_inner().unwrap());

        let dest = tempfile::tempdir().unwrap();
        let (mut bytes, mut entries) = budgets();
        extract_archive(
            dest.path(),
            "run.tar",
            archive.path(),
            &mut bytes,
            &mut entries,
        )
        .unwrap();
        assert!(dest.path().join("ro/child.txt").is_file());
        // Assert the mode directly (not just that the child extracted): root would
        // bypass the OS write check, hiding a still-read-only dir that breaks
        // cleanup and non-root runs.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dest.path().join("ro"))
                .unwrap()
                .permissions()
                .mode();
            assert!(
                mode & 0o200 != 0,
                "scratch dir not owner-writable: {mode:o}"
            );
        }
    }

    #[test]
    fn extract_rejects_too_many_entries() {
        let dest = tempfile::tempdir().unwrap();
        let archive = archive_file(&context_tar()); // two entries
        let (mut bytes, mut entries) = (MAX_RUN_UNPACKED_BYTES, 1);
        assert!(
            extract_archive(
                dest.path(),
                "run.tar",
                archive.path(),
                &mut bytes,
                &mut entries
            )
            .is_err()
        );
    }

    #[test]
    fn extract_rejects_path_traversal() {
        let dest = tempfile::tempdir().unwrap();
        let archive = archive_file(&raw_tar("../escape.txt", "", b'0', b"pwned"));
        let (mut bytes, mut entries) = budgets();
        assert!(
            extract_archive(
                dest.path(),
                "evil.tar",
                archive.path(),
                &mut bytes,
                &mut entries
            )
            .is_err()
        );
    }

    #[test]
    fn extract_rejects_symlink_entry() {
        let dest = tempfile::tempdir().unwrap();
        // A symlink (typeflag '2') pointing outside could redirect later writes.
        let archive = archive_file(&raw_tar("link", "/etc/passwd", b'2', b""));
        let (mut bytes, mut entries) = budgets();
        assert!(
            extract_archive(
                dest.path(),
                "evil.tar",
                archive.path(),
                &mut bytes,
                &mut entries
            )
            .is_err()
        );
    }

    #[test]
    fn limit_reader_errors_when_run_budget_exhausted() {
        let data = [0u8; 100];
        let (mut archive, mut run) = (1_000u64, 50u64);
        let mut reader = LimitReader {
            inner: &data[..],
            archive_remaining: &mut archive,
            run_remaining: &mut run,
        };
        assert!(reader.read_to_end(&mut Vec::new()).is_err());
    }

    #[test]
    fn limit_reader_errors_when_archive_cap_exhausted() {
        let data = [0u8; 100];
        let (mut archive, mut run) = (50u64, 1_000u64);
        let mut reader = LimitReader {
            inner: &data[..],
            archive_remaining: &mut archive,
            run_remaining: &mut run,
        };
        assert!(reader.read_to_end(&mut Vec::new()).is_err());
    }

    #[test]
    fn find_archives_returns_file_and_walks_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.zip"), b"").unwrap();
        std::fs::create_dir(tmp.path().join("sub")).unwrap();
        std::fs::write(tmp.path().join("sub/b.tar"), b"").unwrap();
        std::fs::write(tmp.path().join("notes.txt"), b"").unwrap();

        // A directory: walk it for archives.
        let mut names: Vec<String> = find_archives(tmp.path())
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(names, vec!["a.zip", "b.tar"]);

        // A single archive file passed directly.
        assert_eq!(find_archives(&tmp.path().join("a.zip")).len(), 1);
        // A non-archive file: nothing.
        assert!(find_archives(&tmp.path().join("notes.txt")).is_empty());
    }
}
