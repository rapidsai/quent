// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Build and serve viewers for discovered contexts. Contexts sharing a build
//! spec (same analyzer + pinned commits + format) share one viewer; distinct
//! viewers build in parallel and are announced as they become ready.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener as StdTcpListener};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use backon::{ConstantBuilder, Retryable};
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::task::JoinSet;

use crate::error::{OpenError, Result};
use crate::spec::ViewerSpec;
use crate::wrapper::{self, ADDR_ENV, ROOT_ENV, WRAPPER_PACKAGE};

/// Viewer to build: representative [`ViewerSpec`] plus all contexts sharing it.
pub struct ViewerGroup {
    pub spec: ViewerSpec,
    pub contexts: Vec<PathBuf>,
}

/// Build and serve groups in parallel, announcing each URL when ready, each bound
/// on `host`. Open a browser only for a single viewer. Block until all viewers
/// exit (e.g. Ctrl-C); failed builds do not stop others.
pub async fn open_all(groups: Vec<ViewerGroup>, no_browser: bool, host: IpAddr) -> Result<()> {
    let total: usize = groups.iter().map(|g| g.contexts.len()).sum();
    println!(
        "discovered {total} context(s) -> {} viewer(s)",
        groups.len()
    );
    let open_browser = !no_browser && groups.len() == 1;

    let mut set = JoinSet::new();
    for group in groups {
        set.spawn(async move { open_one(group, open_browser, host).await });
    }

    let mut failures = 0usize;
    while let Some(joined) = set.join_next().await {
        match joined {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                failures += 1;
                eprintln!("viewer failed: {e}");
            }
            Err(e) => {
                failures += 1;
                eprintln!("viewer task error: {e}");
            }
        }
    }
    if failures > 0 {
        Err(OpenError::ViewersFailed { count: failures })
    } else {
        Ok(())
    }
}

/// Build one group's viewer and serve all its contexts.
async fn open_one(group: ViewerGroup, open_browser: bool, host: IpAddr) -> Result<()> {
    let ViewerGroup { spec, contexts } = group;
    let label = format!("{} ({} context(s))", spec.analyzer_package, contexts.len());
    println!("building: {label}");

    let crate_dir = build_dir(&spec)?;
    wrapper::generate(&spec, &crate_dir)?;
    let bin = cargo_build(&crate_dir).await?;
    let output_root = stage_output_root(&crate_dir, &contexts)?;
    let result = serve(&output_root, &bin, &label, open_browser, host).await;
    // Best-effort cleanup of this run's staged root; keep the cached build.
    let _ = std::fs::remove_dir_all(&output_root);
    result
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
/// read from Cargo's JSON output so a custom target dir/triple is handled. Build
/// diagnostics go to `<crate_dir>/build.log` so parallel builds don't interleave;
/// on failure the log's tail is folded into the error.
///
/// The first build fetches the pinned git sources and compiles the embedded UI,
/// which invokes `pnpm`/`node`; both must be on `PATH`. Subsequent builds reuse
/// the cached `crate_dir`.
async fn cargo_build(crate_dir: &Path) -> Result<PathBuf> {
    let log_path = crate_dir.join("build.log");
    let log = std::fs::File::create(&log_path)?;
    let mut child = Command::new("cargo")
        .args([
            "build",
            "--release",
            "--message-format=json-render-diagnostics",
        ])
        .current_dir(crate_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::from(log))
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
            status: format!("{status}; last output:\n{}", log_tail(&log_path)),
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

/// Last 20 lines of a build log, for surfacing why a build failed.
fn log_tail(path: &Path) -> String {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(20)..].join("\n")
}

/// Stage a clean output root, symlinking each `context` under its UUID name.
/// The server scans `<context-uuid>/` directories; isolating requested contexts
/// serves exactly them and avoids unrelated siblings that may use another format.
/// The root is per process so concurrent runs sharing a cached build do not
/// clobber each other.
fn stage_output_root(crate_dir: &Path, contexts: &[PathBuf]) -> Result<PathBuf> {
    let root = crate_dir.join(format!("serve-root-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root)?;
    for context in contexts {
        let context = context.canonicalize()?;
        let name = context.file_name().ok_or_else(|| {
            OpenError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "context path has no final component",
            ))
        })?;
        symlink_dir(&context, &root.join(name))?;
    }
    Ok(root)
}

/// Symlink a context directory into the staged output root.
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

/// Spawn the viewer for `output_root` bound on `host`, announce its URL once it
/// accepts connections, and run until exit.
async fn serve(
    output_root: &Path,
    bin: &Path,
    label: &str,
    open_browser: bool,
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

    if wait_until_ready(reachable).await {
        println!("ready: {label}  {url}");
        if open_browser && let Err(e) = open::that(&url) {
            eprintln!("could not open a browser ({e}); open {url} manually");
        }
    } else {
        eprintln!("warning: {label} did not start listening at {url} within the timeout");
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

/// Poll `addr` until it accepts a connection, returning `false` on timeout.
async fn wait_until_ready(addr: SocketAddr) -> bool {
    (|| async { tokio::net::TcpStream::connect(addr).await })
        .retry(
            ConstantBuilder::default()
                .with_delay(Duration::from_millis(100))
                .with_max_times(50),
        )
        .await
        .is_ok()
}
