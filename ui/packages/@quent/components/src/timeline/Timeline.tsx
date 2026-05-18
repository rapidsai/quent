// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useMemo, useRef } from 'react';
import ReactEChartsComponent from 'echarts-for-react';
import { echarts } from '../lib/echarts';
import type { EChartsOption } from '../lib/echarts';
import type { LineSeriesOption } from 'echarts/charts';
import type { EChartsInstance } from 'echarts-for-react';
import { withOpacity } from '@quent/utils';
import type { TimelineSeriesEntry } from './types';
import { TimelineSeries, TimelineMark, TIMELINE_SPACING, TIMELINE_X_AXIS_ANIMATION } from './types';
import {
  MARK_AREA_BORDER_OPACITY,
  MARK_AREA_FILL_OPACITY,
  MARK_LABEL_TEXT_COLOR,
  ROLLUP_TIMELINE_COLOR_DARK,
  ROLLUP_TIMELINE_COLOR_LIGHT,
  TIMELINE_MONO_FONT,
  useTimelineEchartsTheme,
} from './timelineEchartsTheme';
import { MIN_ZOOM_WINDOW_S, nanosToMs } from '../lib/timeline.utils';
import { useVisibleMaxValue } from './useVisibleMaxValue';
import { useChartConnect } from '../lib/useChartConnect';
import { Opts } from 'echarts-for-react/lib/types';

export const CHART_GROUP = 'timeline-sync-group';
const DIMMED_OPACITY = 0.25;

/**
 * Pointer position over the chart, expressed in coordinates the parent can
 * use to drive a tooltip outside the chart.
 */
export interface TimelineHoverPosition {
  dataIndex: number;
  timestampMs: number;
  clientX: number;
  clientY: number;
}

/** Stacked-area timeline chart backed by ECharts, with zoom sync and optional tooltip. */
export function Timeline({
  startTime,
  durationSeconds,
  series,
  timestamps,
  showTooltip = true,
  marks,
  isDark,
  onHoverChange,
}: {
  startTime: bigint;
  /** Full query duration — used to set xAxis range so dataZoom percentages align across all connected charts */
  durationSeconds: number;
  series: TimelineSeries;
  timestamps: number[];
  showTooltip?: boolean;
  /** Annotation marks rendered as mark areas on the first series */
  marks?: TimelineMark[];
  /** Whether dark mode is active. Passed explicitly to decouple from ThemeContext. */
  isDark: boolean;
  /** Pointer-state callback. */
  onHoverChange?: (position: TimelineHoverPosition | null) => void;
}) {
  const { themeName, textColor, labelBackgroundColor } = useTimelineEchartsTheme(isDark);
  const maxMarkCountRef = useRef(0);

  const seriesOptions = useMemo(() => {
    const sortedEntries = Object.entries(series).sort((a, b) => a[0].localeCompare(b[0]));
    const rollupTimelineColor = isDark ? ROLLUP_TIMELINE_COLOR_DARK : ROLLUP_TIMELINE_COLOR_LIGHT;

    const allSeries: LineSeriesOption[] = sortedEntries.map(([name, seriesData]) => {
      const isOverlay = seriesData.isOverlay ?? false;
      const isDimmed = seriesData.isDimmed ?? false;
      // When an operator is selected, collapse all non-overlay states to a
      // single neutral gray so the operator overlay reads as the figure and
      // everything else recedes as a monotone background.
      const renderColor = isDimmed ? rollupTimelineColor : seriesData.color;

      return {
        name,
        type: 'line',
        stack: isOverlay ? `overlay-total` : 'total',
        step: 'middle',
        symbol: 'circle',
        symbolSize: (value: number[]) => (value[1] === 0 || isOverlay ? 0 : 4),
        hoverAnimation: false,
        showSymbol: false,
        ...TIMELINE_X_AXIS_ANIMATION,
        cursor: 'default',
        data: seriesData.values.map((value, index) => [timestamps[index], value]),
        lineStyle: { width: 0 },
        itemStyle: { color: renderColor },
        areaStyle: {
          color: renderColor,
          opacity: isDimmed ? DIMMED_OPACITY : 1,
        },
        z: isOverlay ? 5 : 2,
        sampling: 'lttb',
        emphasis: {
          disabled: true,
          focus: 'none',
        },
      };
    });

    const markCount = marks?.length ?? 0;
    maxMarkCountRef.current = Math.max(maxMarkCountRef.current, markCount);

    for (let i = 0; i < maxMarkCountRef.current; i++) {
      const m = marks?.[i];
      if (m) {
        const stateColor = m.color;
        const dimmed = m.isDimmed ?? false;
        allSeries.push({
          name: `__mark_${i}`,
          type: 'line',
          step: 'middle',
          data: [
            [m.xStart, 0],
            {
              value: [m.xStart, 1],
              label: {
                show: !dimmed,
                formatter: () =>
                  `${m.label}\n${m.stateName}${m.operatorName ? `\n${m.operatorName}` : ''}`,
                position: [0, -2.5],
                fontSize: 9,
                fontWeight: 500,
                color: MARK_LABEL_TEXT_COLOR,
                backgroundColor: withOpacity(stateColor, 0.85),
                borderRadius: 1,
                padding: [1, 2],
              },
            },
            [m.xEnd, 1],
            [m.xEnd, 0],
          ],
          zlevel: 1,
          label: { show: false },
          symbolSize: 0,
          lineStyle: {
            width: 1,
            color: withOpacity(stateColor, dimmed ? DIMMED_OPACITY : MARK_AREA_BORDER_OPACITY),
          },
          areaStyle: {
            color: withOpacity(stateColor, dimmed ? DIMMED_OPACITY : MARK_AREA_FILL_OPACITY),
            opacity: 1,
          },
          tooltip: { show: false },
          silent: true,
          animation: false,
          yAxisIndex: 1,
        });
      } else {
        allSeries.push({
          name: `__mark_${i}`,
          type: 'line',
          data: [],
          zlevel: 1,
          symbolSize: 0,
          lineStyle: { width: 0 },
          areaStyle: { opacity: 0 },
          tooltip: { show: false },
          silent: true,
          animation: false,
          yAxisIndex: 1,
        });
      }
    }

    return allSeries;
  }, [series, timestamps, marks, isDark]);

  const formatAxisValue = useMemo(() => {
    const firstEntry: TimelineSeriesEntry | undefined = Object.values(series)[0];
    return (v: number) => firstEntry?.formatter(v, 0) ?? String(v);
  }, [series]);

  const startTimeMs = useMemo(() => nanosToMs(startTime), [startTime]);

  const maxValue = useVisibleMaxValue(series, timestamps, startTimeMs);

  const yAxisOptions = useMemo(
    () => [
      {
        type: 'value',
        min: 0,
        // Adds a 10% padding to the top of the bars
        max: (value: { max: number }) => value.max * 1.1 || 1,
        splitNumber: 1,
        show: true,
        axisLabel: { show: false },
      },
      {
        type: 'value',
        show: false,
        min: 0,
        max: 1,
        gridIndex: 0,
      },
    ],
    []
  );

  const xAxisOptions = useMemo(
    () => ({
      boundaryGap: false,
      type: 'time',
      animation: false,
      show: true,
      min: startTimeMs,
      max: startTimeMs + durationSeconds * 1_000,
      axisLine: { onZero: true },
      axisLabel: { show: false },
      axisPointer: {
        show: true,
        type: 'line',
        animation: false,
        label: { show: false },
      },
    }),
    [startTimeMs, durationSeconds]
  );

  const gridOptions = useMemo(() => ({ ...TIMELINE_SPACING }), []);

  const minZoomSpanPct = useMemo(() => {
    if (durationSeconds <= 0) return 0;
    return Math.min(100, (MIN_ZOOM_WINDOW_S / durationSeconds) * 100);
  }, [durationSeconds]);

  const minZoomSpanPctRef = useRef(minZoomSpanPct);
  minZoomSpanPctRef.current = minZoomSpanPct;
  const atZoomLimitRef = useRef(false);

  // ECharts' built-in tooltip is reduced to crosshair only (`showContent: false`).
  // Tooltip content is rendered by the parent via `onHoverChange` — keeping
  // `connect()` mirroring `showTip` harmless (only the crosshair paints,
  // never tooltip DOM.
  const eChartOptions: EChartsOption = useMemo(() => {
    return {
      animation: false,
      tooltip: {
        show: true,
        showContent: false,
        trigger: 'axis',
        transitionDuration: 0,
      },
      title: {
        left: 'center',
      },
      axisPointer: {
        link: [{ xAxisIndex: 'all' }],
      },
      grid: gridOptions,
      xAxis: xAxisOptions,
      yAxis: yAxisOptions,
      series: seriesOptions,
      dataZoom: [
        {
          type: 'slider',
          show: false,
          realtime: true,
          filterMode: 'none',
          minSpan: minZoomSpanPct,
        },
        {
          type: 'inside',
          zoomLock: true,
          zoomOnMouseWheel: false,
          moveOnMouseWheel: false,
          throttle: 30,
          filterMode: 'none',
        },
        {
          type: 'inside',
          zoomOnMouseWheel: 'shift',
          moveOnMouseMove: false,
          moveOnMouseWheel: false,
          throttle: 30,
          filterMode: 'none',
          minSpan: minZoomSpanPct,
        },
      ],
    } as EChartsOption;
  }, [gridOptions, minZoomSpanPct, xAxisOptions, yAxisOptions, seriesOptions]);

  const isDraggingRef = useRef(false);

  // `onChartReady` runs exactly once per chart instance, so its closures
  // capture the initial values of `showTooltip` / `onHoverChange`. Refs let
  // those closures read the current values on every event without re-binding
  // listeners or making `onChartReady` re-run.
  const showTooltipRef = useRef(showTooltip);
  showTooltipRef.current = showTooltip;
  const onHoverChangeRef = useRef(onHoverChange);
  onHoverChangeRef.current = onHoverChange;
  // The listeners attached in `onChartReady` close over `timestamps` for
  // bin snapping; mirror it into a ref so they always see the current array
  // (zoom changes can replace it) without re-binding.
  const timestampsRef = useRef(timestamps);
  timestampsRef.current = timestamps;

  const onChartReady = useCallback((instance: EChartsInstance) => {
    const dom = instance.getDom();
    const outsideTimelineViz = (e: PointerEvent) => {
      const rect = dom.getBoundingClientRect();
      const offsetX = e.clientX - rect.left;
      const offsetY = e.clientY - rect.top;
      return (
        offsetX < TIMELINE_SPACING.left ||
        offsetX > rect.width - TIMELINE_SPACING.right ||
        offsetY < TIMELINE_SPACING.top ||
        offsetY > rect.height - TIMELINE_SPACING.bottom
      );
    };

    // Pointer activity is reported up via `onHoverChange`. The parent owns
    // the tooltip-rendering / shared-state concerns; this component only
    // converts pointer pixels into a snapped bin index so the parent can
    // sample series data without re-doing the search.
    const reportHover = (e: PointerEvent) => {
      if (!showTooltipRef.current) return;
      if (isDraggingRef.current) return;
      if (instance.isDisposed?.()) return;
      const rect = dom.getBoundingClientRect();
      const offsetX = e.clientX - rect.left;
      // Don't report hover if the pointer is outside the timeline
      if (outsideTimelineViz(e)) {
        onHoverChangeRef.current?.(null);
        return;
      }
      let tsMs: number;
      try {
        const v = instance.convertFromPixel({ xAxisIndex: 0 }, offsetX);
        if (v == null || !isFinite(v as number)) return;
        tsMs = v as number;
      } catch {
        return;
      }
      const idx = snapToBinIndex(timestampsRef.current, tsMs);
      if (idx < 0) return;
      onHoverChangeRef.current?.({
        dataIndex: idx,
        timestampMs: tsMs,
        clientX: e.clientX,
        clientY: e.clientY,
      });
    };

    const reportLeave = () => onHoverChangeRef.current?.(null);

    dom.addEventListener('pointermove', reportHover);
    dom.addEventListener('pointerleave', reportLeave);
    dom.addEventListener('pointercancel', reportLeave);
    dom.addEventListener('pointerdown', () => {
      isDraggingRef.current = true;
      reportLeave();
    });
    dom.addEventListener('pointerup', () => {
      isDraggingRef.current = false;
    });

    // Update atZoomLimitRef from ECharts' datazoom event, which fires synchronously
    // within the same dispatch tick as the wheel handler — no React render-cycle lag.
    instance.on('datazoom', () => {
      const opt = instance.getOption() as { dataZoom?: Array<{ start?: number; end?: number }> };
      const dz = opt.dataZoom?.[0];
      if (dz != null) {
        const spanPct = (dz.end ?? 100) - (dz.start ?? 0);
        atZoomLimitRef.current = spanPct <= minZoomSpanPctRef.current * 1.01;
      }
    });

    // Pass non-shift wheel events through to the page for normal scrolling.
    // Without this, ECharts' inside dataZoom calls preventDefault on all wheel events.
    // When at the zoom limit, also block shift+wheel-in before ECharts sees it —
    // ECharts converts a blocked zoom into a pan, so we must stop it at the source.
    dom.addEventListener(
      'wheel',
      e => {
        if (!e.shiftKey) {
          e.stopPropagation();
        } else if (e.deltaY < 0 && atZoomLimitRef.current) {
          e.stopPropagation();
        }
      },
      { capture: true, passive: true }
    );

    // Prevent the browser from handling shift+wheel-in when ECharts can't zoom further
    dom.addEventListener(
      'wheel',
      e => {
        if (e.shiftKey && e.deltaY < 0) e.preventDefault();
      },
      { passive: false }
    );
  }, []);

  // If this Timeline is unmounted while the pointer is over it (e.g. a tree
  // row is virtualized away mid-hover, or ResourceTimeline swaps to a
  // skeleton on refetch), no DOM `pointerleave` will fire. Tell the parent
  // explicitly so it can clear any tooltip state it owns.
  useEffect(() => {
    return () => {
      onHoverChangeRef.current?.(null);
    };
  }, []);

  const style = useMemo(() => ({ width: '100%', height: '100%' }), []);
  const opts = useMemo(() => ({ renderer: 'svg' }) as Opts, []);

  const { handleChartReady } = useChartConnect({
    durationSeconds,
    chartGroup: CHART_GROUP,
    onReady: onChartReady,
  });

  return (
    <div className="relative w-full h-full">
      {maxValue != null && (
        <span
          className="absolute z-[8] pointer-events-none text-[10px] leading-none rounded-sm px-1 py-0.5"
          style={{
            top: TIMELINE_SPACING.top + 1,
            left: TIMELINE_SPACING.left + 1,
            fontFamily: TIMELINE_MONO_FONT,
            color: textColor,
            background: labelBackgroundColor,
          }}
        >
          {formatAxisValue(maxValue)}
        </span>
      )}
      <ReactEChartsComponent
        echarts={echarts}
        theme={themeName}
        opts={opts}
        option={eChartOptions}
        style={style}
        onChartReady={handleChartReady}
        notMerge={false}
        lazyUpdate={false}
        replaceMerge={['series']}
        autoResize={false}
      />
    </div>
  );
}

/**
 * Snap a continuous x-axis time (ms) to the nearest bin index by binary
 * search. dataIndex from echarts cannot be trusted at tiny bin sizes.
 */
function snapToBinIndex(timestamps: number[], ts: number): number {
  const n = timestamps.length;
  if (n === 0) return -1;
  if (n === 1) return 0;
  let lo = 0;
  let hi = n - 1;
  while (lo < hi) {
    const mid = (lo + hi) >>> 1;
    if ((timestamps[mid] ?? 0) < ts) lo = mid + 1;
    else hi = mid;
  }
  if (lo > 0) {
    const a = timestamps[lo - 1] ?? 0;
    const b = timestamps[lo] ?? 0;
    if (Math.abs(a - ts) < Math.abs(b - ts)) return lo - 1;
  }
  return lo;
}
