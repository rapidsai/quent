// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useRef, useState } from 'react';
import type { EChartsInstance } from 'echarts-for-react';
import { usePlayheadLineTimeMs } from '@quent/hooks';

export function usePlayheadLinePixel(
  instance: EChartsInstance | null,
  xAxisIndex = 0
): number | null {
  const [pixelX, setPixelX] = useState<number | null>(null);
  const timestampMs = usePlayheadLineTimeMs();
  const timestampMsRef = useRef(timestampMs);
  timestampMsRef.current = timestampMs;

  const recompute = useCallback(() => {
    const ts = timestampMsRef.current;
    if (!instance || ts === null) {
      setPixelX(null);
      return;
    }
    try {
      const pixel = instance.convertToPixel({ xAxisIndex }, ts);
      setPixelX(typeof pixel === 'number' && Number.isFinite(pixel) ? pixel : null);
    } catch {
      setPixelX(null);
    }
  }, [instance, xAxisIndex]);

  // Recompute when the timestamp atom changes.
  useEffect(() => {
    recompute();
  }, [timestampMs, recompute]);

  // Re-attach ECharts listeners when the instance changes; recompute on
  // zoom/resize so the overlay stays aligned with the x-axis.
  // dataZoom covers zoom/pan; finished fires after resize.
  useEffect(() => {
    if (instance) {
      instance.on('dataZoom', recompute);
      instance.on('finished', recompute);
    }
    recompute();
    return () => {
      if (instance) {
        try {
          instance.off('dataZoom', recompute);
          instance.off('finished', recompute);
        } catch {
          // Instance may already be disposed on cleanup.
        }
      }
    };
  }, [instance, recompute]);

  return pixelX;
}
