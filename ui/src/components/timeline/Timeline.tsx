// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useMemo, useRef } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import ReactECharts from 'echarts-for-react/lib/core';
import { echarts } from '@/lib/echarts';
import type { EChartsOption } from '@/lib/echarts';
import type { LineSeriesOption } from 'echarts/charts';
import type { EChartsInstance } from 'echarts-for-react';
import { useAtomValue } from 'jotai';
import { TooltipContent } from './TimelineTooltip';
import { withOpacity } from '@/services/colors';
import type { TimelineSeriesEntry } from './types';
import {
  TimelineSeries,
  TimelineMark,
  DEFAULT_TIMELINE_HEIGHT,
  TIMELINE_SPACING,
  TIMELINE_X_AXIS_ANIMATION,
} from './types';
import { connectChart, MIN_ZOOM_WINDOW_S, nanosToMs } from '@/lib/timeline.utils';
import { useTimelineChartColors, TIMELINE_MONO_FONT } from './useTimelineChartColors';
import { zoomRangeAtom } from '@/atoms/timeline';

export const CHART_GROUP = 'timeline-sync-group';
const DIMMED_OPACITY = 0.25;

export function Timeline({
  startTime,
  durationSeconds,
  series,
  timestamps,
  height = DEFAULT_TIMELINE_HEIGHT,
  showTooltip = true,
  marks,
}: {
  startTime: bigint;
  /** Full query duration — used to set xAxis range so dataZoom percentages align across all connected charts */
  durationSeconds: number;
  series: TimelineSeries;
  timestamps: number[];
  height?: number;
  showTooltip?: boolean;
  /** Annotation marks rendered as mark areas on the first series */
  marks?: TimelineMark[];
}) {
  const {
    timelineMarkupColor,
    gridBorderColor,
    gridBackgroundColor,
    markAreaFillOpacity,
    markAreaBorderOpacity,
    markLabelTextColor,
  } = useTimelineChartColors();

  const zoomRange = useAtomValue(zoomRangeAtom);
  const windowMsRef = useRef(0);
  windowMsRef.current = (zoomRange.end - zoomRange.start) * 1000;

  const maxMarkCountRef = useRef(0);

  const seriesOptions = useMemo(() => {
    const sortedEntries = Object.entries(series).sort((a, b) => a[0].localeCompare(b[0]));

    const allSeries: LineSeriesOption[] = sortedEntries.map(([name, seriesData]) => {
      const color = seriesData.color;
      const isOverlay = seriesData.isOverlay ?? false;
      const isDimmed = seriesData.isDimmed ?? false;

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
        itemStyle: { color },
        areaStyle: {
          color,
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
                position: [0, -5],
                fontSize: 9,
                fontWeight: 500,
                color: markLabelTextColor,
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
            color: withOpacity(stateColor, dimmed ? DIMMED_OPACITY : markAreaBorderOpacity),
          },
          areaStyle: {
            color: withOpacity(stateColor, dimmed ? DIMMED_OPACITY : markAreaFillOpacity),
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
  }, [series, timestamps, marks, markAreaFillOpacity, markAreaBorderOpacity, markLabelTextColor]);

  const yAxisFormatter = useMemo(() => {
    const firstEntry: TimelineSeriesEntry | undefined = Object.values(series)[0];
    return (v: number) => firstEntry?.formatter(v, 0) ?? ((v: number) => `${v}`);
  }, [series]);

  const yAxisOptions = useMemo(
    () => [
      {
        type: 'value',
        min: 0,
        // Adds a 10% padding to the top of the bars
        max: (value: { max: number }) => value.max * 1.1 || 1,
        splitNumber: 1,
        show: true,
        axisLine: {
          show: true,
          lineStyle: { color: gridBorderColor },
        },
        axisTick: { show: false },
        splitLine: { show: false },
        axisLabel: {
          show: true,
          margin: 8,
          fontSize: 10,
          color: timelineMarkupColor,
          fontFamily: TIMELINE_MONO_FONT,
          formatter: yAxisFormatter,
        },
      },
      {
        type: 'value',
        show: false,
        min: 0,
        max: 1,
        gridIndex: 0,
      },
    ],
    [gridBorderColor, timelineMarkupColor, yAxisFormatter]
  );

  const startTimeMs = useMemo(() => nanosToMs(startTime), [startTime]);

  const xAxisOptions = useMemo(
    () => ({
      boundaryGap: false,
      type: 'time',
      animation: false,
      show: true,
      min: startTimeMs,
      max: startTimeMs + durationSeconds * 1_000,
      axisLine: {
        show: true,
        onZero: true,
        lineStyle: { color: gridBorderColor },
      },
      axisTick: { show: false },
      axisLabel: { show: false },
      splitLine: {
        show: false,
      },
      axisPointer: {
        show: true,
        type: 'line',
        animation: false,
        label: { show: false },
        lineStyle: {
          type: 'dashed',
          color: timelineMarkupColor,
        },
      },
    }),
    [timelineMarkupColor, gridBorderColor, startTimeMs, durationSeconds]
  );

  const gridOptions = useMemo(
    () => ({
      ...TIMELINE_SPACING,
      backgroundColor: gridBackgroundColor,
      borderWidth: 1,
      borderColor: gridBorderColor,
      show: true,
    }),
    [gridBorderColor, gridBackgroundColor]
  );

  const minZoomSpanPct = useMemo(() => {
    if (durationSeconds <= 0) return 0;
    return Math.min(100, (MIN_ZOOM_WINDOW_S / durationSeconds) * 100);
  }, [durationSeconds]);

  // Kept in sync via the ECharts datazoom event (no React render-cycle lag) so the
  // capture listener can reliably block shift+wheel-in the moment the limit is reached.
  const minZoomSpanPctRef = useRef(minZoomSpanPct);
  minZoomSpanPctRef.current = minZoomSpanPct;
  const atZoomLimitRef = useRef(false);

  const eChartOptions: EChartsOption = useMemo(() => {
    return {
      animation: false,
      tooltip: {
        show: true,
        showContent: showTooltip,
        trigger: 'axis',
        transitionDuration: 0,
        backgroundColor: 'transparent',
        borderWidth: 0,
        padding: 0,
        textStyle: {},
        confine: true,
        appendToBody: true,
        formatter: function (hoveredSeries: unknown) {
          if (isDraggingRef.current) return '';
          if (!Array.isArray(hoveredSeries) || hoveredSeries.length === 0) return '';
          const timestamp = Number(hoveredSeries[0].axisValue);
          const seriesValues = hoveredSeries
            .filter(
              (p: { seriesName: string; data?: number[] }) =>
                !p.seriesName.startsWith('__mark_') && p.data != null
            )
            .map((p: { color: string; seriesName: string; data: number[] }) => {
              return {
                color: p.color,
                name: p.seriesName,
                value: p.data[1],
                isOverlay: series[p.seriesName]?.isOverlay ?? false,
                isDimmed: series[p.seriesName]?.isDimmed ?? false,
              };
            });
          const activeMarks = marks
            ?.filter(m => timestamp >= m.xStart && timestamp <= m.xEnd)
            .map(m => ({ label: m.label, stateName: m.stateName, color: m.color }));
          const fmt = Object.values(series)[0]?.formatter;
          return renderToStaticMarkup(
            <TooltipContent
              timestamp={timestamp}
              series={seriesValues}
              startTime={startTime}
              fmt={fmt}
              windowMs={windowMsRef.current}
              activeMarks={activeMarks && activeMarks.length > 0 ? activeMarks : undefined}
            />
          );
        },
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
  }, [
    showTooltip,
    gridOptions,
    minZoomSpanPct,
    xAxisOptions,
    yAxisOptions,
    seriesOptions,
    startTime,
    series,
    marks,
  ]);

  const instanceRef = useRef<EChartsInstance | null>(null);
  const isDraggingRef = useRef(false);

  const handleChartReady = useCallback((instance: EChartsInstance) => {
    instanceRef.current = instance;
    connectChart(instance, CHART_GROUP, false);

    // Update atZoomLimitRef synchronously from the ECharts datazoom event, which fires
    // within the same event-dispatch tick as the wheel handler — before any React render.
    // This avoids the one-tick stale-state window that windowMsRef.current has.
    instance.on('datazoom', () => {
      const opt = instance.getOption() as { dataZoom?: Array<{ start?: number; end?: number }> };
      const dz = opt.dataZoom?.[0];
      if (dz != null) {
        const spanPct = (dz.end ?? 100) - (dz.start ?? 0);
        atZoomLimitRef.current = spanPct <= minZoomSpanPctRef.current * 1.01;
      }
    });

    const dom = instance.getDom();
    dom.addEventListener('pointerdown', () => {
      isDraggingRef.current = true;
      instance.dispatchAction({ type: 'hideTip' });
    });
    dom.addEventListener('pointerup', () => {
      isDraggingRef.current = false;
    });

    // Let non-shift wheel events pass through to the page for normal scrolling.
    // Without this, echarts' inside dataZoom calls preventDefault on all wheel events.
    // Also block shift+wheel-in when at the zoom limit so ECharts can't convert the
    // blocked zoom into a pan (zoomLock:true converts excess zoom delta into pan).
    // atZoomLimitRef is kept current by the datazoom event listener above, which fires
    // in the same synchronous tick as the ECharts canvas handler — no render-cycle lag.
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

    // Bubble-phase (fires after ECharts' canvas listener). Prevents the browser from handling
    // shift+wheel zoom-in when ECharts can't act (e.g. zoom limit reached, pan disabled).
    dom.addEventListener(
      'wheel',
      e => {
        if (e.shiftKey && e.deltaY < 0) e.preventDefault();
      },
      { passive: false }
    );
  }, []);

  return (
    <ReactECharts
      echarts={echarts}
      option={eChartOptions}
      style={{ width: '100%', height: `${height}px` }}
      onChartReady={handleChartReady}
      notMerge={false}
      lazyUpdate={false}
      replaceMerge={['series']}
    />
  );
}
