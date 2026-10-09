// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

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
