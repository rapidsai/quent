// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use pyo3::prelude::*;

#[allow(unused)]
mod models {
    include!(concat!(env!("OUT_DIR"), "/models.rs"));
}

#[pymodule]
fn quent_bench_python(module: &Bound<'_, PyModule>) -> PyResult<()> {
    models::register(module)
}
