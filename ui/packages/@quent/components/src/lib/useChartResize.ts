// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useRef } from 'react';
import type { EChartsInstance } from 'echarts-for-react';

/**
 * Wires up the same real-time ResizeObserver used by `useChartConnect` for
 * charts that don't need to join the timeline connect group.
 *
 * Pass `autoResize={false}` to `ReactEChartsComponent` and forward
 * `handleChartReady` to its `onChartReady` prop.
 */
export function useChartResize() {
  const instanceRef = useRef<EChartsInstance | null>(null);
  const resizeCleanupRef = useRef<(() => void) | null>(null);

  const handleChartReady = useCallback((instance: EChartsInstance) => {
    instanceRef.current = instance;
    resizeCleanupRef.current?.();
    resizeCleanupRef.current = null;
    const dom = instance.getDom?.() as HTMLElement | null | undefined;
    if (dom && typeof ResizeObserver !== 'undefined') {
      // Skip first fire to preserve entrance animations (mirrors useChartConnect).
      let initial = true;
      const observer = new ResizeObserver(() => {
        if (initial) {
          initial = false;
          return;
        }
        try {
          instance.resize({ width: 'auto', height: 'auto' });
        } catch {
          // Instance disposed between layout and callback.
        }
      });
      observer.observe(dom);
      resizeCleanupRef.current = () => observer.disconnect();
    }
  }, []);

  useEffect(
    () => () => {
      resizeCleanupRef.current?.();
      resizeCleanupRef.current = null;
    },
    []
  );

  return { handleChartReady, instanceRef };
}
