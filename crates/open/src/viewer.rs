// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generate, build, and serve the viewer crate for a [`ViewerSpec`], then open a
//! browser at the served URL.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener as StdTcpListener};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use backon::{ConstantBuilder, Retryable};
use tokio::io::AsyncReadExt;
use tokio::process::Command;

use crate::error::{OpenError, Result};
use crate::spec::ViewerSpec;
use crate::wrapper::{self, ADDR_ENV, ROOT_ENV, WRAPPER_PACKAGE};

/// Generate, build, and serve the viewer for `spec`, bound on `host`. Blocks
/// serving until the viewer process exits (e.g. Ctrl-C). Opens a browser unless
/// `no_browser`.
pub async fn open(
    spec: &ViewerSpec,
    no_browser: bool,
    print_url: bool,
    host: IpAddr,
) -> Result<()> {
    let crate_dir = build_dir(spec)?;
    wrapper::generate(spec, &crate_dir)?;
    let bin = cargo_build(&crate_dir).await?;
    let output_root = stage_output_root(&crate_dir, &spec.root)?;
    let result = serve(&output_root, &bin, no_browser, print_url, host).await;
    // Best-effort cleanup of this run's staged root (the cached build is kept).
    let _ = std::fs::remove_dir_all(&output_root);
    result
}

/// Stage a clean output root containing only the requested `context`, symlinked
/// under its own UUID name. The server scans an output root of `<context-uuid>/`
/// directories; isolating to one context serves exactly what was asked and avoids
/// tripping over sibling contexts that may use a different format.
///
/// The root is unique per process so concurrent runs sharing a cached build dir
/// do not clobber each other's staged root.
fn stage_output_root(crate_dir: &Path, context: &Path) -> Result<PathBuf> {
    let context = context.canonicalize()?;
    let name = context.file_name().ok_or_else(|| {
        OpenError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "context path has no final component",
        ))
    })?;
    let root = crate_dir.join(format!("serve-root-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root)?;
    symlink_dir(&context, &root.join(name))?;
    Ok(root)
}

/// Symlink the context directory into the staged output root.
#[cfg(unix)]
fn symlink_dir(src: &Path, link: &Path) -> Result<()> {
    std::os::unix::fs::symlink(src, link)?;
    Ok(())
}

#[cfg(not(unix))]
fn symlink_dir(_src: &Path, _link: &Path) -> Result<()> {
    Err(OpenError::Io(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "serving local artifacts requires symlink support",
    )))
}

/// Cache dir for this viewer's generated crate/build, keyed by
/// [`ViewerSpec::cache_key`] under the user cache dir so identical specs are reused.
fn build_dir(spec: &ViewerSpec) -> Result<PathBuf> {
    let base = dirs::cache_dir().ok_or(OpenError::NoCacheDir)?;
    Ok(base
        .join("quent")
        .join("open")
        .join("builds")
        .join(spec.cache_key()))
}

/// Run `cargo build --release` in `crate_dir` and return the built binary path,
/// read from Cargo's JSON output so a custom target dir/triple is handled.
/// Diagnostics stream to stderr so the user still sees build progress.
///
/// The first build fetches the pinned git sources and compiles the embedded UI,
/// which invokes `pnpm`/`node`; both must be on `PATH`. Subsequent builds reuse
/// the cached `crate_dir`.
async fn cargo_build(crate_dir: &Path) -> Result<PathBuf> {
    let mut child = Command::new("cargo")
        .args([
            "build",
            "--release",
            "--message-format=json-render-diagnostics",
        ])
        .current_dir(crate_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|source| OpenError::Spawn {
            what: "cargo build".into(),
            source,
        })?;

    let mut json = Vec::new();
    child
        .stdout
        .take()
        .expect("piped stdout")
        .read_to_end(&mut json)
        .await?;
    let status = child.wait().await?;
    if !status.success() {
        return Err(OpenError::Build {
            status: status.to_string(),
        });
    }
    wrapper_executable(&json).ok_or_else(|| OpenError::Build {
        status: format!("cargo build reported no `{WRAPPER_PACKAGE}` executable"),
    })
}

/// Find the wrapper binary's path in cargo's `--message-format=json`
/// `compiler-artifact` messages (avoids assuming a target-dir layout).
fn wrapper_executable(stdout: &[u8]) -> Option<PathBuf> {
    std::str::from_utf8(stdout).ok()?.lines().find_map(|line| {
        let msg: serde_json::Value = serde_json::from_str(line).ok()?;
        let is_wrapper =
            msg["reason"] == "compiler-artifact" && msg["target"]["name"] == WRAPPER_PACKAGE;
        is_wrapper
            .then(|| msg["executable"].as_str().map(PathBuf::from))
            .flatten()
    })
}

/// Spawn the built viewer serving `output_root` bound to `host`, print/open its
/// URL, and run until it exits.
async fn serve(
    output_root: &Path,
    bin: &Path,
    no_browser: bool,
    print_url: bool,
    host: IpAddr,
) -> Result<()> {
    let addr = free_port(host)?;
    // An unspecified host (`0.0.0.0`/`::`) is not browseable; show and probe the
    // matching loopback instead (the server may be bound v6-only on `::`).
    let reachable = match addr.ip() {
        IpAddr::V4(ip) if ip.is_unspecified() => (Ipv4Addr::LOCALHOST, addr.port()).into(),
        IpAddr::V6(ip) if ip.is_unspecified() => (Ipv6Addr::LOCALHOST, addr.port()).into(),
        _ => addr,
    };
    let url = format!("http://{reachable}/");

    let mut child = Command::new(bin)
        .env(ROOT_ENV, output_root)
        .env(ADDR_ENV, addr.to_string())
        .spawn()
        .map_err(|source| OpenError::Spawn {
            what: "viewer".into(),
            source,
        })?;

    if print_url {
        println!("{url}");
    }
    if !no_browser {
        // Wait for the server to accept connections before opening the browser.
        wait_until_ready(reachable).await;
        if let Err(e) = open::that(&url) {
            eprintln!("could not open a browser ({e}); open {url} manually");
        }
    }

    let status = child.wait().await?;
    if !status.success() {
        return Err(OpenError::ViewerExited {
            status: status.to_string(),
        });
    }
    Ok(())
}

/// Reserve a free TCP port on `host`, returning the full bind address; the small
/// race before the viewer binds it is acceptable for a local dev tool.
fn free_port(host: IpAddr) -> Result<SocketAddr> {
    let listener = StdTcpListener::bind((host, 0))?;
    Ok(listener.local_addr()?)
}

/// Poll `addr` until it accepts a connection (server up) or a few seconds pass,
/// retrying on a fixed interval.
async fn wait_until_ready(addr: SocketAddr) {
    let _ = (|| async { tokio::net::TcpStream::connect(addr).await })
        .retry(
            ConstantBuilder::default()
                .with_delay(Duration::from_millis(100))
                .with_max_times(50),
        )
        .await;
}
