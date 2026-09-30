// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

#[allow(unused)]
pub mod empty {
    include!(concat!(env!("OUT_DIR"), "/empty.rs"));
}

#[allow(unused)]
pub mod u8 {
    include!(concat!(env!("OUT_DIR"), "/u8.rs"));
}

#[allow(unused)]
pub mod u64 {
    include!(concat!(env!("OUT_DIR"), "/u64.rs"));
}

#[allow(unused)]
pub mod short_string {
    include!(concat!(env!("OUT_DIR"), "/short_string.rs"));
}

#[allow(unused)]
pub mod long_string {
    include!(concat!(env!("OUT_DIR"), "/long_string.rs"));
}

#[allow(unused)]
pub mod all {
    include!(concat!(env!("OUT_DIR"), "/all.rs"));
}
