// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from 'vitest';
import { renderHook } from '@testing-library/react';
import { useMinZoomSpanPct } from './useMinZoomSpanPct';
import { MIN_ZOOM_WINDOW_S } from './timeline.utils';

describe('useMinZoomSpanPct', () => {
  it('converts the minimum zoom window to a percentage of the query duration', () => {
    const { result } = renderHook(() => useMinZoomSpanPct(10));
    expect(result.current).toBeCloseTo(MIN_ZOOM_WINDOW_S * 10, 12);
  });

  it('caps the minimum span at the full query duration', () => {
    const { result } = renderHook(() => useMinZoomSpanPct(MIN_ZOOM_WINDOW_S / 2));
    expect(result.current).toBe(100);
  });

  it.each([0, -1])('returns zero for non-positive duration %s', duration => {
    const { result } = renderHook(() => useMinZoomSpanPct(duration));
    expect(result.current).toBe(0);
  });
});
