import { useCallback, useMemo, useRef } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import ReactECharts from 'echarts-for-react/lib/core';
import { echarts } from '@/lib/echarts';
import type { EChartsOption } from '@/lib/echarts';
import type { LineSeriesOption } from 'echarts/charts';
import type { EChartsInstance } from 'echarts-for-react';
import { useAtomValue } from 'jotai';
import { TooltipContent } from './TimelineTooltip';
import { createStripePattern, getColorForKey, withOpacity } from '@/services/colors';
import { formatBytes } from '@/services/formatters';
import {
  TimelineSeries,
  TimelineMark,
  DEFAULT_TIMELINE_HEIGHT,
  TIMELINE_SPACING,
  TIMELINE_X_AXIS_ANIMATION,
} from './types';
import { connectChart, nanosToMs } from '@/lib/timeline.utils';
import { useTimelineChartColors } from './useTimelineChartColors';
import { zoomRangeAtom } from '@/atoms/timeline';

export const CHART_GROUP = 'timeline-sync-group';

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
          color: isOverlay
            ? {
                image: createStripePattern(color),
                repeat: 'repeat',
              }
            : color,
          opacity: 1,
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
        const stateColor = getColorForKey(m.stateName);
        allSeries.push({
          name: `__mark_${i}`,
          type: 'line',
          step: 'middle',
          data: [
            [m.xStart, 0],
            {
              value: [m.xStart, 1],
              label: {
                show: true,
                formatter: () => m.label,
                position: [0, -5],
                fontSize: 9,
                fontWeight: 500,
                color: markLabelTextColor,
                backgroundColor: withOpacity(stateColor, 1),
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
            color: withOpacity(stateColor, markAreaBorderOpacity),
          },
          areaStyle: {
            color: withOpacity(stateColor, markAreaFillOpacity),
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
          // TODO(joe): This needs to be dynamic, not always bytes but looks nice for now
          formatter: (value: number) => {
            return formatBytes(value, 0);
          },
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
    [gridBorderColor, timelineMarkupColor]
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
              };
            });
          const activeMarks = marks
            ?.filter(m => timestamp >= m.xStart && timestamp <= m.xEnd)
            .map(m => ({ label: m.label, stateName: m.stateName }));
          return renderToStaticMarkup(
            <TooltipContent
              timestamp={timestamp}
              series={seriesValues}
              startTime={startTime}
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
        { type: 'slider', show: false, realtime: true, filterMode: 'none' },
        {
          type: 'inside',
          zoomLock: true,
          zoomOnMouseWheel: false,
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
        },
      ],
    } as EChartsOption;
  }, [
    showTooltip,
    gridOptions,
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
    dom.addEventListener(
      'wheel',
      e => {
        if (!e.shiftKey) e.stopPropagation();
      },
      { capture: true, passive: true }
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
