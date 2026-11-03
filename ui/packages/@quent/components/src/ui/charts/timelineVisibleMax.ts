// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { TimelineSeries } from './timelineTypes';

/** Maximum stacked value in the visible window. */
export function computeVisibleMaxValue(
  series: TimelineSeries,
  timestamps: number[],
  zoomStartMs: number,
  zoomEndMs: number
): number | null {
  const allEntries = Object.values(series);
  const overlayEntries = allEntries.filter(entry => entry.isOverlay && !entry.isDimmed);
  const entries =
    overlayEntries.length > 0
      ? overlayEntries
      : allEntries.filter(entry => !entry.isDimmed && !entry.isOverlay);
  if (!entries.length || !entries[0]?.values.length) {
    return null;
  }
  let max = 0;
  const binDurationMs = (entries[0]?.binDuration ?? 0) * 1_000;
  for (let index = 0; index < entries[0].values.length; index++) {
    const timestamp = timestamps[index];
    if (
      timestamp === undefined ||
      timestamp + binDurationMs <= zoomStartMs ||
      timestamp >= zoomEndMs
    ) {
      continue;
    }
    const sum = entries.reduce((total, entry) => total + (entry.values[index] ?? 0), 0);
    max = Math.max(max, sum);
  }
  return max > 0 ? max : null;
}
