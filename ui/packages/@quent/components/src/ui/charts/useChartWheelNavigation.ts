// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useRef } from 'react';
import type { EChartsOption, EChartsType } from 'echarts';
import type { DataZoomComponentOption } from 'echarts/components';

const ZOOM_LIMIT_FLOAT_TOLERANCE = 1.01;

export interface ChartHorizontalPadding {
  left: number;
  right: number;
}

function getDataZoomState(instance: EChartsType): DataZoomComponentOption | undefined {
  const dataZoom = (instance.getOption() as EChartsOption).dataZoom;
  return Array.isArray(dataZoom) ? dataZoom[0] : dataZoom;
}

/** Adds native vertical scrolling, horizontal panning, and minimum-zoom guarding. */
export function useChartWheelNavigation(
  minZoomSpanPct: number,
  horizontalPadding: ChartHorizontalPadding = { left: 0, right: 0 }
) {
  const minZoomSpanPctRef = useRef(minZoomSpanPct);
  // eslint-disable-next-line react-hooks/refs -- latest-value mirror, only read from callbacks/effects
  minZoomSpanPctRef.current = minZoomSpanPct;
  const cleanupRef = useRef<(() => void) | null>(null);

  const attachWheelNavigation = useCallback(
    (instance: EChartsType, wheelTarget: HTMLElement = instance.getDom()) => {
      cleanupRef.current?.();

      const isAtZoomLimit = () => {
        if (instance.isDisposed?.()) {
          return false;
        }
        const dataZoom = getDataZoomState(instance);
        if (!dataZoom) {
          return false;
        }
        const spanPct = (dataZoom.end ?? 100) - (dataZoom.start ?? 0);
        return spanPct <= minZoomSpanPctRef.current * ZOOM_LIMIT_FLOAT_TOLERANCE;
      };

      const handleWheel = (event: WheelEvent) => {
        const isHorizontalScroll =
          event.deltaX !== 0 && Math.abs(event.deltaX) > Math.abs(event.deltaY);
        const hasZoomModifier = event.shiftKey || event.ctrlKey || event.metaKey;

        if (hasZoomModifier && !isHorizontalScroll) {
          if (event.deltaY < 0 && isAtZoomLimit()) {
            event.preventDefault();
            event.stopPropagation();
          }
          return;
        }

        event.stopPropagation();
        if (!isHorizontalScroll || instance.isDisposed?.()) {
          return;
        }

        event.preventDefault();
        const dataZoom = getDataZoomState(instance);
        if (!dataZoom) {
          return;
        }

        const currentStart = dataZoom.start ?? 0;
        const currentEnd = dataZoom.end ?? 100;
        const spanPct = currentEnd - currentStart;
        const rect = instance.getDom().getBoundingClientRect();
        const usableWidth = Math.max(
          1,
          rect.width - horizontalPadding.left - horizontalPadding.right
        );
        const deltaPct = (event.deltaX / usableWidth) * spanPct;
        const newStart = Math.max(0, Math.min(100 - spanPct, currentStart + deltaPct));

        instance.dispatchAction({
          type: 'dataZoom',
          dataZoomIndex: 0,
          start: newStart,
          end: newStart + spanPct,
        });
      };

      wheelTarget.addEventListener('wheel', handleWheel, { capture: true, passive: false });
      const cleanup = () => {
        wheelTarget.removeEventListener('wheel', handleWheel, { capture: true });
        if (cleanupRef.current === cleanup) {
          cleanupRef.current = null;
        }
      };
      cleanupRef.current = cleanup;
      return cleanup;
    },
    [horizontalPadding.left, horizontalPadding.right]
  );

  useEffect(
    () => () => {
      cleanupRef.current?.();
      cleanupRef.current = null;
    },
    []
  );

  return attachWheelNavigation;
}
