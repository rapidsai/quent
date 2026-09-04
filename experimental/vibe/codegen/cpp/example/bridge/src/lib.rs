// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

#[allow(unused, clippy::all)]
mod bridge {
    include!(concat!(env!("OUT_DIR"), "/bridge_mod.rs"));
}

#[cfg(test)]
mod tests {
    unsafe extern "C" {
        fn quent_demo_cpp_smoke() -> std::ffi::c_int;
    }

    #[test]
    fn compiles_links_and_runs_cpp_facade() {
        // The build script compiles this function from the public C++ example.
        assert_eq!(unsafe { quent_demo_cpp_smoke() }, 0);
    }
}
