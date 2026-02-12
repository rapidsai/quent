import { useMemo } from 'react';
import ReactECharts from 'echarts-for-react/lib/core';
import { echarts } from '@/lib/echarts';
import type { EChartsOption } from '@/lib/echarts';
import { withOpacity } from '@/services/colors';
import { formatDuration } from '@/services/formatters';
import { connectChart } from '@/lib/timeline.utils';
import { TIMELINE_SPACING } from './types';

const TIMELINE_MARKUP_COLOR = '#808080';
const GRID_BORDER_COLOR = withOpacity(TIMELINE_MARKUP_COLOR, 0.2);
const CONTROLLER_HEIGHT = 50;

type TimelineControllerProps = {
  /** Start time in nanoseconds (bigint) */
  startTime: bigint;
  /** Duration in seconds */
  durationSeconds: number;
  /** Number of bins to match child timelines */
  numBins?: number;
  height?: number;
};

export function TimelineController({
  startTime,
  durationSeconds,
  numBins = 100,
  height = CONTROLLER_HEIGHT,
}: TimelineControllerProps) {
  // Start time in milliseconds for formatting
  const startTimeMillis = useMemo(() => Number(startTime / 1_000_000n), [startTime]);

  // Generate timestamps matching child timelines
  const timestamps = useMemo(() => {
    const binDurationMs = (durationSeconds * 1000) / numBins;
    return Array.from({ length: numBins }, (_, i) =>
      Math.round(startTimeMillis + i * binDurationMs)
    );
  }, [startTimeMillis, durationSeconds, numBins]);

  // Create two series: one for static axis display, one for dataZoom control
  const seriesOptions = useMemo(
    () => [
      {
        name: 'static-display',
        type: 'line' as const,
        xAxisIndex: 0,
        data: Array(numBins).fill(0),
        showSymbol: false,
        lineStyle: { width: 0 },
        areaStyle: { opacity: 0 },
        silent: true,
      },
      {
        name: 'zoom-control',
        type: 'line' as const,
        xAxisIndex: 1,
        data: Array(numBins).fill(0),
        showSymbol: false,
        lineStyle: { width: 0 },
        areaStyle: { opacity: 0 },
        silent: true,
      },
    ],
    [numBins]
  );

  // Static x-axis (index 0): shows labels, ticks, gridlines - not affected by dataZoom
  const staticXAxisOptions = useMemo(
    () => ({
      data: timestamps,
      boundaryGap: false,
      type: 'category' as const,
      show: true,
      axisLine: {
        show: true,
        lineStyle: { color: GRID_BORDER_COLOR },
      },
      axisTick: { show: false },
      axisLabel: {
        show: true,
        fontSize: 10,
        color: TIMELINE_MARKUP_COLOR,
        formatter: (value: number) => formatDuration(value - startTimeMillis),
      },
      splitLine: {
        show: true,
        lineStyle: {
          color: GRID_BORDER_COLOR,
          type: 'solid' as const,
        },
      },
    }),
    [timestamps, startTimeMillis]
  );

  // Hidden x-axis (index 1): controlled by dataZoom, no visible elements
  const zoomXAxisOptions = useMemo(
    () => ({
      data: timestamps,
      boundaryGap: false,
      type: 'category' as const,
      show: false,
      axisLine: { show: false },
      axisTick: { show: false },
      axisLabel: { show: false },
      splitLine: { show: false },
    }),
    [timestamps]
  );

  const yAxisOptions = useMemo(
    () => ({
      type: 'value',
      show: false,
      min: 0,
      max: 1,
      splitLine: { show: false },
    }),
    []
  );

  const gridOptions = useMemo(
    () => ({
      ...TIMELINE_SPACING,
      bottom: 20, // Make room for x-axis labels
      backgroundColor: withOpacity(TIMELINE_MARKUP_COLOR, 0.05),
      borderWidth: 1,
      borderColor: GRID_BORDER_COLOR,
      show: true,
    }),
    []
  );

  const eChartOptions: EChartsOption = useMemo(() => {
    return {
      tooltip: { show: false },
      axisPointer: {
        link: [{ xAxisIndex: 'all' }],
      },
      grid: gridOptions,
      dataZoom: [
        {
          type: 'slider',
          show: true,
          xAxisIndex: [1], // Only control the hidden zoom axis (index 1), not the static display axis (index 0)
          realtime: true,
          height: height - 25,
          top: 5,
          brushSelect: true,
          handleSize: '100%',
          handleStyle: {
            color: withOpacity(TIMELINE_MARKUP_COLOR, 0.3),
          },
          fillerColor: withOpacity(TIMELINE_MARKUP_COLOR, 0.1),
          borderColor: 'transparent',
          backgroundColor: 'transparent',
          dataBackground: {
            lineStyle: { opacity: 0 },
            areaStyle: { opacity: 0 },
          },
          selectedDataBackground: {
            lineStyle: { opacity: 0 },
            areaStyle: { opacity: 0 },
          },
          moveHandleSize: 5,
          labelFormatter: (_: number, valueStr: string) => {
            const raw = Number(valueStr);
            const minTs = timestamps[0] ?? startTimeMillis;
            const maxTs = timestamps[timestamps.length - 1] ?? startTimeMillis;
            const clamped = Math.max(minTs, Math.min(maxTs, raw));
            return formatDuration(clamped - startTimeMillis);
          },
          emphasis: {
            handleStyle: {
              color: withOpacity(TIMELINE_MARKUP_COLOR, 0.5),
            },
          },
        },
        {
          type: 'inside',
          xAxisIndex: [1], // Only control the hidden zoom axis
          realtime: true,
          zoomOnMouseWheel: true,
          moveOnMouseMove: true,
          moveOnMouseWheel: false,
        },
      ],
      xAxis: [staticXAxisOptions, zoomXAxisOptions],
      yAxis: yAxisOptions,
      series: seriesOptions,
    } as EChartsOption;
  }, [
    seriesOptions,
    yAxisOptions,
    staticXAxisOptions,
    zoomXAxisOptions,
    gridOptions,
    height,
    startTimeMillis,
    timestamps,
  ]);

  return (
    <ReactECharts
      echarts={echarts}
      option={eChartOptions}
      style={{ width: '100%', height: `${height}px` }}
      onChartReady={connectChart}
      notMerge={false}
      lazyUpdate
      opts={{ renderer: 'canvas' }}
    />
  );
}
