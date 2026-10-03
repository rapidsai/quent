// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Linux process-exit regression: glibc destroys main-thread Rust TLS before
//! calling atexit handlers, as in cuDF's late NVTX-emitting global destructor.

use std::cell::RefCell;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use nvtx::sys::ffi;
use quent_instrumentation::EventCallback;
use uuid::Uuid;

thread_local! {
    static TLS_PROBE: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

static ACTIVE_CLEANUP: AtomicBool = AtomicBool::new(false);
static LATE_CALLS: AtomicUsize = AtomicUsize::new(0);

const LATE_CLEANUP_COMPLETED: &[u8] = b"NVTX emitted after TLS destruction\n";

extern "C" fn late_nvtx() {
    // Verify the subprocess actually reached the shutdown phase under test.
    if TLS_PROBE.try_with(|_| ()).is_ok() {
        return;
    }
    // SAFETY: NVTX callbacks remain installed; the C string and zero-initialized
    // attribute struct (integer, union and pointer fields) are valid for each
    // call. A null domain selects the default domain through the CORE2 API.
    unsafe {
        let mut attr: ffi::nvtxEventAttributes_t = std::mem::zeroed();
        attr.version = ffi::NVTX_VERSION as u16;
        attr.size = std::mem::size_of_val(&attr) as u16;
        ffi::nvtxMarkA(c"late mark".as_ptr());
        let domain = ffi::nvtxDomainCreateA(c"late domain".as_ptr());
        let string = ffi::nvtxDomainRegisterStringA(domain, c"late string".as_ptr());
        ffi::nvtxDomainMarkEx(domain, &attr);
        let push = ffi::nvtxRangePushA(c"late default range".as_ptr());
        let pop = ffi::nvtxRangePop();
        let domain_push = ffi::nvtxDomainRangePushEx(domain, &attr);
        let domain_pop = ffi::nvtxDomainRangePop(domain);
        let range = ffi::nvtxRangeStartA(c"late async range".as_ptr());
        ffi::nvtxRangeEnd(range);
        ffi::nvtxDomainDestroy(domain);
        // The callbacks remain installed after capture ends. Handle creation
        // still works, and push/pop reports NVTX_NO_PUSH_POP_TRACKING (-2)
        // when its thread-local nesting state has been destroyed.
        if domain.is_null()
            || string.is_null()
            || range == 0
            || [push, pop, domain_push, domain_pop] != [-2; 4]
        {
            libc::_exit(1);
        }
        if ACTIVE_CLEANUP.load(Ordering::Relaxed) && LATE_CALLS.load(Ordering::Relaxed) != 11 {
            libc::_exit(1);
        }
        // Avoid Rust buffered output during teardown.
        libc::write(
            libc::STDOUT_FILENO,
            LATE_CLEANUP_COMPLETED.as_ptr().cast(),
            LATE_CLEANUP_COMPLETED.len(),
        );
    }
}

fn main() {
    let mode = std::env::args().nth(1);
    if matches!(
        mode.as_deref(),
        Some("--late-cleanup-child" | "--active-cleanup-child")
    ) {
        TLS_PROBE.with(|probe| probe.borrow_mut().push(1));
        // SAFETY: the handler has the required C ABI and stays linked until exit.
        assert_eq!(unsafe { libc::atexit(late_nvtx) }, 0);

        if mode.as_deref() == Some("--active-cleanup-child") {
            nvtx_injection::install_hook(|_| {
                LATE_CALLS.fetch_add(1, Ordering::Relaxed);
            })
            .unwrap();
            // Initialize range nesting state before TLS destruction and keep
            // the process-wide hook active during atexit.
            drop(nvtx::LocalRange::new(c"initialize cache"));
            assert_eq!(LATE_CALLS.swap(0, Ordering::Relaxed), 2);
            ACTIVE_CLEANUP.store(true, Ordering::Relaxed);
            return;
        }

        let calls = Arc::new(AtomicUsize::new(0));
        let weak_calls = Arc::downgrade(&calls);
        let sink = EventCallback::new({
            let calls = Arc::clone(&calls);
            move |_| {
                calls.fetch_add(1, Ordering::Relaxed);
            }
        });
        // Exercise the real owner's install and cleanup path on the main OS
        // thread. Its push/pop initializes the injection library's RANGE_DEPTH.
        nvtx_example::run_capture(Uuid::now_v7(), sink).expect("capture");
        assert_eq!(calls.load(Ordering::Relaxed), 6);
        drop(calls);
        assert!(
            weak_calls.upgrade().is_none(),
            "capture retained the exporter"
        );
        nvtx::mark(c"after context drop");
        drop(nvtx::LocalRange::new(c"after context drop"));
        assert!(nvtx_injection::install_hook(|_| unreachable!()).is_err());
        return;
    }

    for mode in ["--late-cleanup-child", "--active-cleanup-child"] {
        let output = Command::new(std::env::current_exe().unwrap())
            .arg(mode)
            .output()
            .expect("run shutdown subprocess");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{}: {stderr}", output.status);
        assert_eq!(
            output.stdout, LATE_CLEANUP_COMPLETED,
            "late cleanup did not run"
        );
        // The original bug also exited successfully: catch_unwind contains the
        // AccessError but still prints panic diagnostics. Exit status alone misses it.
        assert!(
            stderr.is_empty(),
            "late NVTX cleanup wrote to stderr: {stderr}"
        );
    }
}
