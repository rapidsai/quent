// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_schema::{Annotations, DataType, Field, Identifier, Record, builder::RecordBuilder};

use crate::{process_path, thread_path};

/// Return the canonical process record.
///
/// Linux [`pid_t`][linux-pid-t] and macOS [`pid_t`][macos-pid-t] are `I32`,
/// while Windows [`GetCurrentProcessId`] returns a `U32` [`DWORD`]. The record
/// assumes only valid process IDs are recorded; these are nonnegative and fit
/// in `U32`, matching [`std::process::id`].
///
/// [linux-pid-t]: https://man7.org/linux/man-pages/man3/pid_t.3type.html
/// [macos-pid-t]: https://developer.apple.com/documentation/oslog/oslogentryfromprocess/processidentifier
/// [`DWORD`]: https://learn.microsoft.com/en-us/windows/win32/winprog/windows-data-types#dword
/// [`GetCurrentProcessId`]: https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getcurrentprocessid
pub fn process_record() -> Record {
    RecordBuilder::new(process_path())
        .with_field(Field::new(
            Identifier::try_new("native_id").expect("canonical field name is valid"),
            DataType::U32,
            Annotations::default(),
        ))
        .build()
        .expect("canonical process record is valid")
}

/// Return the canonical thread record.
///
/// Linux [`gettid`] returns an `I32` `pid_t`, macOS [`threadIdentifier`] is
/// `U64`, and Windows [`GetCurrentThreadId`] returns a `U32` [`DWORD`]. The
/// record assumes only valid OS thread IDs are recorded; Linux IDs are
/// nonnegative, so widening the Linux and Windows values to `U64` is lossless.
/// [`std::thread::ThreadId`] is opaque and is not an OS thread ID.
///
/// [`gettid`]: https://man7.org/linux/man-pages/man2/gettid.2.html
/// [`threadIdentifier`]: https://developer.apple.com/documentation/oslog/oslogentryfromprocess/threadidentifier
/// [`DWORD`]: https://learn.microsoft.com/en-us/windows/win32/winprog/windows-data-types#dword
/// [`GetCurrentThreadId`]: https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getcurrentthreadid
pub fn thread_record() -> Record {
    RecordBuilder::new(thread_path())
        .with_field(Field::new(
            Identifier::try_new("native_id").expect("canonical field name is valid"),
            DataType::U64,
            Annotations::default(),
        ))
        .build()
        .expect("canonical thread record is valid")
}
