// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

export const AGG_MODES = ['value', 'sum', 'mean', 'min', 'max', 'stdev'] as const;

export type AggMode = (typeof AGG_MODES)[number];
