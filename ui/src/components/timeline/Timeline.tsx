import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import ReactECharts from 'echarts-for-react/lib/core';
import { echarts } from '@/lib/echarts';
import type { EChartsOption } from '@/lib/echarts';
import type { LineSeriesOption } from 'echarts/charts';
import type { EChartsInstance } from 'echarts-for-react';
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
  marks,
}: {
  startTime: bigint;
  series: TimelineSeries;
  timestamps: number[];
  height?: number;
  showTooltip?: boolean;
  /** When set, the chart renders as a standalone window (no connect/dataZoom) bounded by these limits */
  xAxisRange?: XAxisRange;
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
        emphasis: {
          disabled: true,
          focus: 'none',
        },
      };
    });

    if (marks && marks?.length > 0) {
      for (const m of marks) {
        const stateColor = getColorForKey(m.stateName);
        allSeries.push({
          name: `__mark_${m.label}_${m.stateName}`,
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
      }
    }

    return allSeries;
  }, [series, timestamps, marks, markAreaFillOpacity, markAreaBorderOpacity, markLabelTextColor]);

  const [prevSeriesCount, setPrevSeriesCount] = useState(seriesOptions.length);
  const notMerge = seriesOptions.length < prevSeriesCount;
  useEffect(() => {
    setPrevSeriesCount(seriesOptions.length);
  }, [seriesOptions.length]);

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
        animation: false,
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
          const seriesValues = hoveredSeries
            .filter((p: { seriesName: string }) => p.seriesName !== '__marks__')
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
  }, [
    showTooltip,
    gridOptions,
    xAxisOptions,
    yAxisOptions,
    seriesOptions,
    xAxisRange,
    startTime,
    series,
    marks,
  ]);

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
      notMerge={notMerge}
      lazyUpdate
    />
  );
}
