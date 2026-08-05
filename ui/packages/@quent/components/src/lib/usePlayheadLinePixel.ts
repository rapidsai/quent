// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useRef, useState } from 'react';
import type { MutableRefObject } from 'react';
import type { EChartsInstance } from 'echarts-for-react';
import { usePlayheadLineTimeMs } from '@quent/hooks';

/**
 * @param readySignal - Bump when the chart instance is recreated (e.g. theme
 *   switch) so the hook re-attaches its ECharts listeners to the new instance.
 */
export function usePlayheadLinePixel(
  instanceRef: MutableRefObject<EChartsInstance | null>,
  xAxisIndex = 0,
  readySignal = 0
): number | null {
  const [pixelX, setPixelX] = useState<number | null>(null);
  const timestampMs = usePlayheadLineTimeMs();
  const timestampMsRef = useRef(timestampMs);
  timestampMsRef.current = timestampMs;

  const recompute = useCallback(() => {
    const instance = instanceRef.current;
    const ts = timestampMsRef.current;
    if (!instance || ts === null) {
      setPixelX(null);
      return;
    }
    try {
      const pixel = instance.convertToPixel({ xAxisIndex }, ts);
      setPixelX(pixel != null && isFinite(pixel) ? pixel : null);
    } catch {
      setPixelX(null);
    }
  }, [instanceRef, xAxisIndex]);

  // Recompute when the timestamp atom changes.
  useEffect(() => {
    recompute();
  }, [timestampMs, recompute]);

  // Re-attach ECharts listeners when the instance changes; recompute on
  // zoom/resize so the overlay stays aligned with the x-axis.
  // dataZoom covers zoom/pan; finished fires after resize.
  useEffect(() => {
    const instance = instanceRef.current;
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
  }, [instanceRef, xAxisIndex, readySignal, recompute]);

  return pixelX;
}
