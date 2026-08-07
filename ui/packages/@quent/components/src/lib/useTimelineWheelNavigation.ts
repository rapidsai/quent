// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useRef } from 'react';
import type { EChartsOption, EChartsType } from 'echarts';
import type { DataZoomComponentOption } from 'echarts/components';
import { TIMELINE_SPACING } from '../timeline/types';

const ZOOM_LIMIT_FLOAT_TOLERANCE = 1.01;
const WHEEL_ZOOM_FACTOR = 1.1;

function getDataZoomState(instance: EChartsType): DataZoomComponentOption | undefined {
  const dataZoom = (instance.getOption() as EChartsOption).dataZoom;
  return Array.isArray(dataZoom) ? dataZoom[0] : dataZoom;
}

/**
 * Adds native vertical scrolling, horizontal trackpad panning, and minimum-zoom guarding.
 * Returns a cleanup handle for charts removed while the hook remains mounted.
 */
export function useTimelineWheelNavigation(minZoomSpanPct: number) {
  const minZoomSpanPctRef = useRef(minZoomSpanPct);
  minZoomSpanPctRef.current = minZoomSpanPct;
  const cleanupRef = useRef<(() => void) | null>(null);

  const attachWheelNavigation = useCallback(
    (instance: EChartsType, wheelTarget: HTMLElement = instance.getDom()) => {
      cleanupRef.current?.();

      const isAtZoomLimit = () => {
        if (instance.isDisposed?.()) return false;
        const dataZoom = getDataZoomState(instance);
        if (!dataZoom) return false;
        const spanPct = (dataZoom.end ?? 100) - (dataZoom.start ?? 0);
        return spanPct <= minZoomSpanPctRef.current * ZOOM_LIMIT_FLOAT_TOLERANCE;
      };

      const handleWheel = (event: WheelEvent) => {
        const isHorizontalScroll =
          event.deltaX !== 0 && Math.abs(event.deltaX) > Math.abs(event.deltaY);

        if (event.shiftKey && !isHorizontalScroll) {
          if (instance.isDisposed?.()) return;
          const dataZoom = getDataZoomState(instance);
          if (!dataZoom) return;

          event.preventDefault();
          event.stopPropagation();

          const currentStart = dataZoom.start ?? 0;
          const currentEnd = dataZoom.end ?? 100;
          const currentSpan = currentEnd - currentStart;
          const zoomingIn = event.deltaY < 0;
          const unclampedNextSpan = zoomingIn
            ? currentSpan / WHEEL_ZOOM_FACTOR
            : currentSpan * WHEEL_ZOOM_FACTOR;
          const nextSpan = Math.max(minZoomSpanPctRef.current, Math.min(100, unclampedNextSpan));

          if (zoomingIn && isAtZoomLimit()) return;

          const rect = instance.getDom().getBoundingClientRect();
          const usableWidth = Math.max(
            1,
            rect.width - TIMELINE_SPACING.left - TIMELINE_SPACING.right
          );
          const localX =
            event.clientX > 0 ? event.clientX - rect.left - TIMELINE_SPACING.left : usableWidth / 2;
          const anchorPct = Math.max(0, Math.min(1, localX / usableWidth));
          const anchorValue = currentStart + currentSpan * anchorPct;
          const unclampedStart = anchorValue - nextSpan * anchorPct;
          const newStart = Math.max(0, Math.min(100 - nextSpan, unclampedStart));

          instance.dispatchAction({
            type: 'dataZoom',
            dataZoomIndex: 0,
            start: newStart,
            end: newStart + nextSpan,
          });
          return;
        }

        event.stopPropagation();
        if (!isHorizontalScroll) return;
        if (instance.isDisposed?.()) return;

        event.preventDefault();
        const dataZoom = getDataZoomState(instance);
        if (!dataZoom) return;

        const currentStart = dataZoom.start ?? 0;
        const currentEnd = dataZoom.end ?? 100;
        const spanPct = currentEnd - currentStart;
        const rect = instance.getDom().getBoundingClientRect();
        const usableWidth = Math.max(
          1,
          rect.width - TIMELINE_SPACING.left - TIMELINE_SPACING.right
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
        if (cleanupRef.current === cleanup) cleanupRef.current = null;
      };
      cleanupRef.current = cleanup;
      return cleanup;
    },
    []
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
