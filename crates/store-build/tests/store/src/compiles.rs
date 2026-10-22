// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Checks that generated code compiles with optional build settings disabled.

#![allow(unused_imports, dead_code)]

include!(concat!(env!("OUT_DIR"), "/compile_options.rs"));
