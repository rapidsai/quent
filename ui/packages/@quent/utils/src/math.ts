// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

export function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function isValidRange(min: number, max: number): boolean {
  return Number.isFinite(min) && Number.isFinite(max) && max > min;
}

/** Sign-preserving log1p, so zero and negative measurements stay defined. */
function signedLog1p(n: number): number {
  return Math.sign(n) * Math.log1p(Math.abs(n));
}

/**
 * Normalize linearly to [0, 1], with a neutral midpoint for invalid or
 * constant ranges.
 */
export function normalizeLinearScale(value: number, min: number, max: number): number {
  if (!Number.isFinite(value) || !isValidRange(min, max)) {
    return 0.5;
  }
  return (clamp(value, min, max) - min) / (max - min);
}

/**
 * Normalize logarithmically to [0, 1], with a neutral midpoint for invalid or
 * constant ranges. Signed log1p keeps zero and negative measurements defined.
 */
export function normalizeLogScale(value: number, min: number, max: number): number {
  if (!Number.isFinite(value) || !isValidRange(min, max)) {
    return 0.5;
  }
  const clamped = clamp(value, min, max);
  if (clamped === min) {
    return 0;
  }
  if (clamped === max) {
    return 1;
  }
  const low = signedLog1p(min);
  const range = signedLog1p(max) - low;
  return range > 0 ? clamp((signedLog1p(clamped) - low) / range, 0, 1) : 0.5;
}

/**
 * The value at position `t` (0 to 1) along a log scale over [min, max]; the
 * inverse of `normalizeLogScale`. Returns `min` for invalid or constant ranges.
 */
export function logScaleValueAt(t: number, min: number, max: number): number {
  if (!isValidRange(min, max)) {
    return min;
  }
  const low = signedLog1p(min);
  const x = low + clamp(t, 0, 1) * (signedLog1p(max) - low);
  return Math.sign(x) * Math.expm1(Math.abs(x));
}
