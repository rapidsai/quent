import { useCallback, useMemo, useRef } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import ReactECharts from 'echarts-for-react/lib/core';
import { echarts } from '@/lib/echarts';
import type { EChartsOption } from '@/lib/echarts';
import type { EChartsInstance } from 'echarts-for-react';
import { TooltipContent } from './TimelineTooltip';
import { createStripePattern } from '@/services/colors';
import { formatBytes } from '@/services/formatters';
import {
  TimelineSeries,
  DEFAULT_TIMELINE_HEIGHT,
  TIMELINE_SPACING,
  TIMELINE_X_AXIS_ANIMATION,
} from './types';
import { connectChart } from '@/lib/timeline.utils';
import { useTimelineChartColors } from './useTimelineChartColors';

export const CHART_GROUP = 'timeline-sync-group';

export function Timeline({
  startTime,
  durationSeconds,
  series,
  timestamps,
  height = DEFAULT_TIMELINE_HEIGHT,
  showTooltip = true,
}: {
  startTime: bigint;
  /** Full query duration — used to set xAxis range so dataZoom percentages align across all connected charts */
  durationSeconds: number;
  series: TimelineSeries;
  timestamps: number[];
  height?: number;
  showTooltip?: boolean;
}) {
  const { timelineMarkupColor, gridBorderColor, gridBackgroundColor } = useTimelineChartColors();

  const seriesOptions = useMemo(() => {
    return Object.entries(series)
      .sort((a, b) => a[0].localeCompare(b[0]))
      .map(([name, seriesData]) => {
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
          emphasis: {
            disabled: true,
            focus: 'none',
          },
        };
      });
  }, [series, timestamps]);

  const yAxisOptions = useMemo(
    () => ({
      type: 'value',
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
    }),
    [gridBorderColor, timelineMarkupColor]
  );

  const startTimeMs = useMemo(() => Number(startTime / 1_000_000n), [startTime]);

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
          const seriesValues = hoveredSeries.map(
            (p: { color: string; seriesName: string; data: number[] }) => {
              return {
                color: p.color,
                name: p.seriesName,
                value: p.data[1],
                isOverlay: series[p.seriesName]?.isOverlay ?? false,
              };
            }
          );
          return renderToStaticMarkup(
            <TooltipContent timestamp={timestamp} series={seriesValues} startTime={startTime} />
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
        { type: 'inside', zoomLock: true, zoomOnMouseWheel: false, filterMode: 'none' },
        {
          type: 'inside',
          zoomOnMouseWheel: 'shift',
          moveOnMouseMove: false,
          moveOnMouseWheel: false,
          filterMode: 'none',
        },
      ],
    } as EChartsOption;
  }, [showTooltip, gridOptions, xAxisOptions, yAxisOptions, seriesOptions, startTime, series]);

  const instanceRef = useRef<EChartsInstance | null>(null);
  const isDraggingRef = useRef(false);

  const handleChartReady = useCallback((instance: EChartsInstance) => {
    instanceRef.current = instance;
    connectChart(instance, CHART_GROUP, false);

    instance.getDom().addEventListener('pointerdown', () => {
      isDraggingRef.current = true;
      instance.dispatchAction({ type: 'hideTip' });
    });
    instance.getDom().addEventListener('pointerup', () => {
      isDraggingRef.current = false;
    });
  }, []);

  return (
    <ReactECharts
      echarts={echarts}
      option={eChartOptions}
      style={{ width: '100%', height: `${height}px` }}
      onChartReady={handleChartReady}
      notMerge={false}
      lazyUpdate
    />
  );
}
