// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useRef } from 'react';
import type { MutableRefObject } from 'react';
import type { EChartsInstance } from 'echarts-for-react';
import { useZoomRange } from '@quent/hooks';
import { connectChart } from './timeline.utils';
import { useChartResize } from './useChartResize';

export interface UseChartConnectOptions {
  /** Full query duration in seconds. Used to convert zoomRangeAtom seconds → percent. */
  durationSeconds: number;
  /** Connect-group name. Defaults to the timeline sync group. */
  chartGroup?: string;
  /** Forwarded to {@link connectChart}; enables ECharts' brush-select cursor. */
  activateBrushSelect?: boolean;
  /**
   * Component-specific setup invoked after the standard connect-group join.
   * Use this for DOM listeners, ECharts events, axis-pointer sync, etc.
   */
  onReady?: (instance: EChartsInstance) => void;
}

export interface UseChartConnectResult {
  /** Pass to `<ReactEChartsComponent onChartReady={...} />`. */
  handleChartReady: (instance: EChartsInstance) => void;
  /** Live ref to the active ECharts instance, or null before first ready. */
  instanceRef: MutableRefObject<EChartsInstance | null>;
}

/**
 * Shared `onChartReady` handler for charts that participate in the timeline
 * connect group.
 *
 * Reads the live zoom from `zoomRangeAtom` via {@link useZoomRange} on every
 * render and stashes both the zoom and the current `durationSeconds` in refs,
 * so the returned `handleChartReady` always seeds new instances with the
 * up-to-date values without rebuilding the callback. This is what keeps a
 * user's saved zoom intact across theme switches (which simultaneously dispose
 * and recreate every chart in the group).
 *
 * The component-specific bits — DOM event listeners, axis-pointer sync,
 * ready-tick state, etc. — are passed in via `onReady`, which fires after
 * the standard {@link connectChart} call.
 */
export function useChartConnect({
  durationSeconds,
  chartGroup,
  activateBrushSelect = false,
  onReady,
}: UseChartConnectOptions): UseChartConnectResult {
  const zoomRange = useZoomRange();

  const zoomRangeRef = useRef(zoomRange);
  zoomRangeRef.current = zoomRange;
  const durationSecondsRef = useRef(durationSeconds);
  durationSecondsRef.current = durationSeconds;
  const onReadyRef = useRef(onReady);
  onReadyRef.current = onReady;

  const { handleChartReady: handleResize, instanceRef } = useChartResize();

  const handleChartReady = useCallback(
    (instance: EChartsInstance) => {
      handleResize(instance);
      const dur = durationSecondsRef.current;
      const range = zoomRangeRef.current;
      const zoomPct =
        dur > 0 ? { start: (range.start / dur) * 100, end: (range.end / dur) * 100 } : null;
      connectChart(instance, chartGroup, activateBrushSelect, zoomPct);
      onReadyRef.current?.(instance);
    },
    [handleResize, chartGroup, activateBrushSelect]
  );

  return { handleChartReady, instanceRef };
}
