// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  EDGE_SCALE_TYPE,
  logScaleValueAt,
  normalizeLinearScale,
  normalizeLogScale,
  type EdgeScaleType,
} from '@quent/utils';

/** Bounds of an edge statistic. `logMin` is the smallest positive value (the log scale's start). */
export interface EdgeScaleRange {
  min: number;
  logMin: number;
  max: number;
}

/** Position (0 to 1) of `value` on the chosen scale. */
export function normalizeEdgeValue(
  value: number,
  range: EdgeScaleRange,
  scale: EdgeScaleType
): number {
  return scale === EDGE_SCALE_TYPE.LOG
    ? normalizeLogScale(value, range.logMin, range.max)
    : normalizeLinearScale(value, range.min, range.max);
}

/** The value found halfway along the chosen scale, for labelling a legend. */
export function edgeScaleMidpoint(range: EdgeScaleRange, scale: EdgeScaleType): number {
  return scale === EDGE_SCALE_TYPE.LOG
    ? logScaleValueAt(0.5, range.logMin, range.max)
    : range.min + (range.max - range.min) / 2;
}
