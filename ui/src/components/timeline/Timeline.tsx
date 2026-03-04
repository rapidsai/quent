import { useCallback, useEffect, useMemo, useRef } from 'react';
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
  timestampToIndex,
  registerAxisPointerSync,
  updateAxisPointerBinning,
  unregisterAxisPointerSync,
} from '@/lib/timeline.utils';
import { useTimelineChartColors } from './useTimelineChartColors';

export const CHART_GROUP = 'timeline-sync-group';

export function Timeline({
  series,
  timestamps,
  height = DEFAULT_TIMELINE_HEIGHT,
  showTooltip = true,
  marks,
}: {
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
        symbolSize: (value: number) => (value === 0 || isOverlay ? 0 : 4),
        hoverAnimation: false,
        showSymbol: false,
        ...TIMELINE_X_AXIS_ANIMATION,
        cursor: 'default',
        data: seriesData.values,
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

    const binDuration = timestamps.length >= 2 ? timestamps[1]! - timestamps[0]! : 1;
    const firstTs = timestamps[0] ?? 0;

    if (marks && marks.length > 0) {
      for (const m of marks) {
        const stateColor = getColorForKey(m.stateName);
        const startIdx = timestampToIndex(firstTs, binDuration, timestamps.length, m.xStart);
        const endIdx = timestampToIndex(firstTs, binDuration, timestamps.length, m.xEnd);

        const markData: (number | null | { value: number; label: Record<string, unknown> })[] =
          new Array(timestamps.length).fill(null);

        if (startIdx > 0) markData[startIdx - 1] = 0;
        for (let i = startIdx; i <= endIdx && i < timestamps.length; i++) {
          markData[i] =
            i === startIdx
              ? {
                  value: 1,
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
                }
              : 1;
        }
        if (endIdx + 1 < timestamps.length) markData[endIdx + 1] = 0;

        allSeries.push({
          name: `__mark_${m.label}_${m.stateName}`,
          type: 'line',
          step: 'middle',
          data: markData as (number | null)[],
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

  const yAxisOptions = useMemo(
    () => [
      {
        type: 'value',
        min: 0,
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
      type: 'category',
      boundaryGap: false,
      data: timestamps,
      animation: false,
      show: true,
      axisLine: {
        show: true,
        onZero: true,
        lineStyle: { color: gridBorderColor },
      },
      axisTick: { show: false },
      axisLabel: { show: false },
      splitLine: { show: false },
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
    [timestamps, timelineMarkupColor, gridBorderColor]
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
            .filter(
              (p: { seriesName: string; data?: unknown }) =>
                !p.seriesName.startsWith('__mark_') && p.data != null
            )
            .map((p: { color: string; seriesName: string; data: number }) => {
              return {
                color: p.color,
                name: p.seriesName,
                value: p.data,
                isOverlay: series[p.seriesName]?.isOverlay ?? false,
              };
            });
          const activeMarks = marks
            ?.filter(m => timestamp >= m.xStart && timestamp <= m.xEnd)
            .map(m => ({ label: m.label, stateName: m.stateName }));
          return renderToStaticMarkup(
            <TooltipContent
              timestampSec={timestamp}
              series={seriesValues}
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
    } as EChartsOption;
  }, [showTooltip, gridOptions, xAxisOptions, yAxisOptions, seriesOptions, series, marks]);

  const instanceRef = useRef<EChartsInstance | null>(null);

  const firstTimestamp = timestamps[0] ?? 0;
  const binDuration = timestamps.length >= 2 ? timestamps[1]! - timestamps[0]! : 1;

  const handleChartReady = useCallback(
    (instance: EChartsInstance) => {
      instanceRef.current = instance;
      registerAxisPointerSync(instance, 0, firstTimestamp, binDuration);
    },
    [firstTimestamp, binDuration]
  );

  useEffect(() => {
    if (instanceRef.current) {
      updateAxisPointerBinning(instanceRef.current, firstTimestamp, binDuration);
    }
  }, [firstTimestamp, binDuration]);

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
