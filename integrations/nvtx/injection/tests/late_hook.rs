// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! The Rust hook may be installed after NVTX initializes its C callbacks.

use std::sync::{Arc, Mutex};

use nvtx::sys::ffi;
use nvtx_injection::Record;

#[test]
fn late_hook_receives_subsequent_events() {
    let early_range = unsafe { ffi::nvtxRangeStartA(c"early".as_ptr()) };
    assert_ne!(early_range, 0);
    unsafe { ffi::nvtxRangeEnd(early_range) };

    let events = Arc::new(Mutex::new(Vec::new()));
    nvtx_injection::install_hook({
        let events = Arc::clone(&events);
        move |event| events.lock().unwrap().push(event)
    })
    .unwrap();

    unsafe {
        ffi::nvtxMarkA(c"mark".as_ptr());
        assert_eq!(ffi::nvtxRangePushA(c"push".as_ptr()), 0);
        assert_eq!(ffi::nvtxRangePop(), 0);
        let range = ffi::nvtxRangeStartA(c"start".as_ptr());
        assert_ne!(range, 0);
        ffi::nvtxRangeEnd(range);
    }

    let events = events.lock().unwrap();
    assert_eq!(events.len(), 5);
    assert!(matches!(events[0], Record::Mark { .. }));
    assert!(matches!(events[1], Record::RangePush { .. }));
    assert!(matches!(events[2], Record::RangePop { .. }));
    assert!(matches!(events[3], Record::RangeStart { .. }));
    assert!(matches!(events[4], Record::RangeEnd { .. }));
}
