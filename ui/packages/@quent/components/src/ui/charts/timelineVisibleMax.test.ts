// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import type { TimelineSeriesEntry } from './timelineTypes';
import { computeVisibleMaxValue } from './timelineVisibleMax';

function entry(color: string, overrides: Partial<TimelineSeriesEntry> = {}): TimelineSeriesEntry {
  return {
    color,
    values: [],
    binDuration: 1,
    formatter: String,
    ...overrides,
  };
}

describe('computeVisibleMaxValue', () => {
  it('uses active overlays instead of dimmed base series', () => {
    const series = {
      base: entry('#f00', { values: [100, 100], isDimmed: true }),
      running: entry('#0f0', { values: [2, 3], isOverlay: true }),
      waiting: entry('#00f', { values: [4, 1], isOverlay: true }),
    };
    expect(computeVisibleMaxValue(series, [0, 1000], 0, 2000)).toBe(6);
  });

  it('uses regular series when no overlay is active', () => {
    const series = {
      running: entry('#0f0', { values: [2, 3] }),
      waiting: entry('#00f', { values: [4, 1] }),
    };
    expect(computeVisibleMaxValue(series, [0, 1000], 0, 2000)).toBe(6);
  });
});
