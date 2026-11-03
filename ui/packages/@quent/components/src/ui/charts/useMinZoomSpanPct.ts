// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo } from 'react';

export function getMinZoomSpanPct(durationSeconds: number, minZoomWindowSeconds: number): number {
  if (durationSeconds <= 0) {
    return 0;
  }
  return Math.min(100, (minZoomWindowSeconds / durationSeconds) * 100);
}

/** ECharts `dataZoom.minSpan` for a minimum visible time window. */
export function useMinZoomSpanPct(durationSeconds: number, minZoomWindowSeconds: number): number {
  return useMemo(
    () => getMinZoomSpanPct(durationSeconds, minZoomWindowSeconds),
    [durationSeconds, minZoomWindowSeconds]
  );
}
