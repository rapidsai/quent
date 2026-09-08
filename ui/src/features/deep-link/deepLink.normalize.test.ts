// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { normalizeZoomRange, resolveCapturedZoomRange } from './deepLink.normalize';

describe('timeline viewport normalization', () => {
  it('preserves a valid viewport', () => {
    expect(normalizeZoomRange({ start: 10, end: 40 }, 100)).toEqual({
      range: { start: 10, end: 40 },
      wasAdjusted: false,
    });
  });

  it('clamps a viewport to the query duration', () => {
    expect(normalizeZoomRange({ start: 10, end: 120 }, 100)).toEqual({
      range: { start: 10, end: 100 },
      wasAdjusted: true,
    });
  });

  it('falls back to the full query when clamping collapses the range', () => {
    expect(normalizeZoomRange({ start: 120, end: 140 }, 100)).toEqual({
      range: { start: 0, end: 100 },
      wasAdjusted: true,
    });
  });

  it('captures the full query before timeline state is initialized', () => {
    expect(resolveCapturedZoomRange({ start: 0, end: 0 }, 100)).toEqual({
      start: 0,
      end: 100,
    });
  });
});
