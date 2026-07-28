// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo } from 'react';
import { MIN_ZOOM_WINDOW_S } from './timeline.utils';

/** ECharts `dataZoom.minSpan` that preserves the timeline bin-size floor. */
export function useMinZoomSpanPct(durationSeconds: number): number {
  return useMemo(() => {
    if (durationSeconds <= 0) return 0;
    return Math.min(100, (MIN_ZOOM_WINDOW_S / durationSeconds) * 100);
  }, [durationSeconds]);
}
