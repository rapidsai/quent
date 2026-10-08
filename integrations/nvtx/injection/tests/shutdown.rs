// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Linux process-exit regression: glibc destroys main-thread Rust TLS before
//! calling atexit handlers, as in cuDF's late NVTX-emitting global destructor.

use std::cell::RefCell;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use nvtx::sys::ffi;

thread_local! {
    static TLS_PROBE: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

const LATE_CLEANUP: &[u8] = b"NVTX emitted after TLS destruction\n";

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
        let push = ffi::nvtxRangePushA(c"late default range".as_ptr());
        let pop = ffi::nvtxRangePop();
        let domain_push = ffi::nvtxDomainRangePushEx(std::ptr::null_mut(), &attr);
        let domain_pop = ffi::nvtxDomainRangePop(std::ptr::null_mut());
        if [push, pop, domain_push, domain_pop] != [-2; 4] {
            libc::_exit(1);
        }
        // Avoid Rust buffered output during teardown.
        libc::write(
            libc::STDOUT_FILENO,
            LATE_CLEANUP.as_ptr().cast(),
            LATE_CLEANUP.len(),
        );
    }
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some("--late-cleanup-child") {
        TLS_PROBE.with(|probe| probe.borrow_mut().push(1));
        // SAFETY: the handler has the required C ABI and stays linked until exit.
        assert_eq!(unsafe { libc::atexit(late_nvtx) }, 0);

        let calls = Arc::new(AtomicUsize::new(0));
        let weak_calls = Arc::downgrade(&calls);
        nvtx_injection::install_hook(move |_| {
            if let Some(calls) = weak_calls.upgrade() {
                calls.fetch_add(1, Ordering::Relaxed);
            }
        })
        .expect("install hook");
        // Initialize the injection library and its range-depth TLS on the main thread.
        unsafe {
            assert_eq!(ffi::nvtxRangePushA(c"before cleanup".as_ptr()), 0);
            assert_eq!(ffi::nvtxRangePop(), 0);
        }
        assert_eq!(calls.load(Ordering::Relaxed), 2);
        drop(calls);
        return;
    }

    let output = Command::new(std::env::current_exe().unwrap())
        .arg("--late-cleanup-child")
        .output()
        .expect("run shutdown subprocess");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{}: {stderr}", output.status);
    assert_eq!(output.stdout, LATE_CLEANUP, "late cleanup did not run");
    assert!(
        stderr.is_empty(),
        "late NVTX cleanup wrote to stderr: {stderr}"
    );
}
