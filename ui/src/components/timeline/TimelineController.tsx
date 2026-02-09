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

  // Create empty data series (all zeros) - invisible, just needed for dataZoom to work
  const seriesOptions = useMemo(
    () => [
      {
        name: 'controller',
        type: 'line',
        data: Array(numBins).fill(0),
        showSymbol: false,
        lineStyle: { width: 0 },
        areaStyle: { opacity: 0 },
        silent: true,
      },
    ],
    [numBins]
  );

  const xAxisOptions = useMemo(
    () => ({
      data: timestamps,
      boundaryGap: false,
      type: 'category',
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
          type: 'solid',
        },
      },
    }),
    [timestamps, startTimeMillis]
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
          xAxisIndex: 'all',
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
          labelFormatter: (_: number, valueStr: string) =>
            formatDuration(Number(valueStr) - startTimeMillis),
          emphasis: {
            handleStyle: {
              color: withOpacity(TIMELINE_MARKUP_COLOR, 0.5),
            },
          },
        },
        {
          type: 'inside',
          xAxisIndex: 'all',
          realtime: true,
          zoomOnMouseWheel: true,
          moveOnMouseMove: true,
          moveOnMouseWheel: false,
        },
      ],
      xAxis: xAxisOptions,
      yAxis: yAxisOptions,
      series: seriesOptions,
    } as EChartsOption;
  }, [seriesOptions, yAxisOptions, xAxisOptions, gridOptions, height, startTimeMillis]);

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
