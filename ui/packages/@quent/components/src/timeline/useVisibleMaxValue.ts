// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo } from 'react';
import { useZoomRange } from '@quent/hooks';
import { computeVisibleMaxValue } from '../lib/timeline.utils';
import type { TimelineSeries } from './types';

export function useVisibleMaxValue(
  series: TimelineSeries,
  timestamps: number[],
  startTimeMs: number
): number | null {
  const zoomRange = useZoomRange();
  return useMemo(() => {
    const zoomStartMs = startTimeMs + zoomRange.start * 1000;
    const zoomEndMs = startTimeMs + zoomRange.end * 1000;
    return computeVisibleMaxValue(series, timestamps, zoomStartMs, zoomEndMs);
  }, [series, timestamps, startTimeMs, zoomRange]);
}
