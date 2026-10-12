// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { continuousColor, type ContinuousPaletteName } from './colors';
import { SCALE_TYPE, type ScaleType } from './dagTypes';
import { logScaleValueAt, normalizeLinearScale, normalizeLogScale } from './math';

/** Bounds of a statistic. `logMin` is the smallest positive value (the log scale's start). */
export interface ScaleRange {
  min: number;
  logMin: number;
  max: number;
}

/** Position (0 to 1) of `value` on the chosen scale. */
export function normalizeScaleValue(value: number, range: ScaleRange, scale: ScaleType): number {
  return scale === SCALE_TYPE.LOG
    ? normalizeLogScale(value, range.logMin, range.max)
    : normalizeLinearScale(value, range.min, range.max);
}

/** The value found halfway along the chosen scale, for labelling a legend. */
export function scaleMidpoint(range: ScaleRange, scale: ScaleType): number {
  return scale === SCALE_TYPE.LOG
    ? logScaleValueAt(0.5, range.logMin, range.max)
    : range.min + (range.max - range.min) / 2;
}

/**
 * Bounds of `values`. Non-negative data starts its log scale at the smallest
 * positive value so zeros don't leave most of the scale empty. Signed data
 * keeps its true minimum, because the symmetric log scale already places
 * negatives and zero.
 */
export function computeScaleRange(values: readonly number[]): ScaleRange {
  let min = Infinity;
  let max = -Infinity;
  let smallestPositive = Infinity;
  for (const v of values) {
    min = Math.min(min, v);
    max = Math.max(max, v);
    if (v > 0) {
      smallestPositive = Math.min(smallestPositive, v);
    }
  }
  const logMin = min < 0 || !Number.isFinite(smallestPositive) ? min : smallestPositive;
  return { min, logMin, max };
}

/**
 * Color of `value` on a continuous palette, placed on the chosen scale. Every
 * view that colors by a statistic should go through this so they agree.
 */
export function scaleColor(
  value: number,
  range: ScaleRange,
  scale: ScaleType,
  palette: ContinuousPaletteName,
  isDark: boolean
): string {
  return continuousColor(normalizeScaleValue(value, range, scale), palette, isDark);
}
