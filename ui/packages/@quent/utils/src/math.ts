// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

export function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}
