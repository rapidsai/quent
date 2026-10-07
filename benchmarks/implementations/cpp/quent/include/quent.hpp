// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

#pragma once

#include "rust/cxx.h"

namespace quent_bench {
struct Workload;
struct Measurement;
Measurement measure_quent(const Workload &workload, rust::Str shape,
                          rust::Str exporter, rust::Str output);
} // namespace quent_bench
