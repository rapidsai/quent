import { useCallback, useEffect, useMemo, useRef } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import ReactECharts from 'echarts-for-react/lib/core';
import { echarts } from '@/lib/echarts';
import type { EChartsOption } from '@/lib/echarts';
import type { EChartsInstance } from 'echarts-for-react';
import { TooltipContent } from './TimelineTooltip';
import { withOpacity } from '@/services/colors';
import { formatBytes } from '@/services/formatters';
import {
  TimelineSeries,
  DEFAULT_TIMELINE_HEIGHT,
  TIMELINE_SPACING,
  TIMELINE_X_AXIS_ANIMATION,
} from './types';
import {
  connectChart,
  registerAxisPointerSync,
  unregisterAxisPointerSync,
} from '@/lib/timeline.utils';
import { useTimelineChartColors } from './useTimelineChartColors';

export const CHART_GROUP = 'timeline-sync-group';

export interface XAxisRange {
  /** xAxis min in milliseconds */
  min: number;
  /** xAxis max in milliseconds */
  max: number;
}

export function Timeline({
  startTime,
  series,
  timestamps,
  height = DEFAULT_TIMELINE_HEIGHT,
  showTooltip = true,
  xAxisRange,
}: {
  startTime: bigint;
  series: TimelineSeries;
  timestamps: number[];
  height?: number;
  showTooltip?: boolean;
  /** When set, the chart renders as a standalone window (no connect/dataZoom) bounded by these limits */
  xAxisRange?: XAxisRange;
}) {
  const { timelineMarkupColor, gridBorderColor, gridBackgroundColor } = useTimelineChartColors();

  const seriesOptions = useMemo(() => {
    return (
      Object.entries(series)
        // TODO(joe): How should we sort series within the timeline?
        // Everything alphabetical RN to keep it consistent
        .sort((a, b) => a[0].localeCompare(b[0]))
        .map(([name, seriesData]) => {
          const color = seriesData.color;
          return {
            name,
            type: 'line',
            stack: 'total',
            step: 'middle',
            symbol: 'circle',
            symbolSize: 4,
            hoverAnimation: false,
            showSymbol: false,
            ...TIMELINE_X_AXIS_ANIMATION,
            cursor: 'default',
            data: seriesData.values.map((value, index) => [timestamps[index], value]),
            lineStyle: { width: 0 },
            itemStyle: { color },
            areaStyle: { color: withOpacity(color, 0.9) },
            emphasis: {
              disabled: true,
              focus: 'none',
            },
          };
        })
    );
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

  const xAxisOptions = useMemo(
    () => ({
      boundaryGap: false,
      type: 'time',
      animation: false,
      show: true,
      ...(xAxisRange && { min: xAxisRange.min, max: xAxisRange.max }),
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
        label: { show: false },
        lineStyle: {
          type: 'dashed',
          color: timelineMarkupColor,
        },
      },
    }),
    [timelineMarkupColor, gridBorderColor, xAxisRange]
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
          if (!Array.isArray(hoveredSeries) || hoveredSeries.length === 0) return '';
          const timestamp = Number(hoveredSeries[0].axisValue);
          const seriesValues = hoveredSeries.map(
            (p: { color: string; seriesName: string; data: number[] }) => {
              return {
                color: p.color,
                name: p.seriesName,
                value: p.data[1],
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
      ...(xAxisRange
        ? {}
        : {
            toolbox: {
              show: false,
              feature: { dataZoom: { yAxisIndex: 'none' } },
            },
            dataZoom: [
              {
                type: 'slider',
                show: false,
                realtime: true,
                xAxisIndex: 'all',
                filterMode: 'none',
              },
            ],
          }),
    } as EChartsOption;
  }, [seriesOptions, yAxisOptions, xAxisOptions, gridOptions, startTime, showTooltip, xAxisRange]);

  const instanceRef = useRef<EChartsInstance | null>(null);

  const handleChartReady = useCallback(
    (instance: EChartsInstance) => {
      instanceRef.current = instance;
      if (xAxisRange) {
        registerAxisPointerSync(instance);
      } else {
        connectChart(instance);
        registerAxisPointerSync(instance);
      }
    },
    [xAxisRange]
  );

  useEffect(() => {
    return () => {
      if (instanceRef.current) {
        unregisterAxisPointerSync(instanceRef.current);
        instanceRef.current = null;
      }
    };
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
