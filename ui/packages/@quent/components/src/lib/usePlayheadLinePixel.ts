// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useState } from 'react';
import type { MutableRefObject } from 'react';
import type { EChartsInstance } from 'echarts-for-react';
import { subscribePlayheadLine } from './timeline.utils';

/**
 * Subscribes to playhead line broadcasts and converts the timestamp to a pixel
 * x-coordinate using the provided chart instance ref
 */
export function usePlayheadLinePixel(
  instanceRef: MutableRefObject<EChartsInstance | null>,
  xAxisIndex = 0
): number | null {
  const [pixelX, setPixelX] = useState<number | null>(null);

  useEffect(() => {
    return subscribePlayheadLine(timestampMs => {
      const instance = instanceRef.current;
      if (!instance || timestampMs === null) {
        setPixelX(null);
        return;
      }
      try {
        const pixel = instance.convertToPixel({ xAxisIndex }, timestampMs);
        setPixelX(pixel != null && isFinite(pixel) ? pixel : null);
      } catch {
        setPixelX(null);
      }
    });
  }, [instanceRef, xAxisIndex]);

  return pixelX;
}
