// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useRef, useState } from 'react';
import type { MutableRefObject } from 'react';
import type { EChartsInstance } from 'echarts-for-react';
import { subscribePlayheadLine } from './timeline.utils';

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
  const latestTimestampMsRef = useRef<number | null>(null);

  useEffect(() => {
    const recompute = () => {
      const instance = instanceRef.current;
      const ts = latestTimestampMsRef.current;
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
    };

    const unsubPlayhead = subscribePlayheadLine(timestampMs => {
      latestTimestampMsRef.current = timestampMs;
      recompute();
    });

    // dataZoom covers zoom/pan; finished fires after resize.
    const instance = instanceRef.current;
    if (instance) {
      instance.on('dataZoom', recompute);
      instance.on('finished', recompute);
    }

    recompute();

    return () => {
      unsubPlayhead();
      if (instance) {
        try {
          instance.off('dataZoom', recompute);
          instance.off('finished', recompute);
        } catch {
          // Instance may already be disposed on cleanup.
        }
      }
    };
  }, [instanceRef, xAxisIndex, readySignal]);

  return pixelX;
}
