import { useMemo } from 'react';
import ReactECharts from 'echarts-for-react/lib/core';
import { echarts } from '@/lib/echarts';
import type { EChartsOption } from '@/lib/echarts';
import { withOpacity } from '@/services/colors';
import { formatDuration } from '@/services/formatters';
import { buildBinnedTimelineSeries, connectChart } from '@/lib/timeline.utils';
import { TIMELINE_SPACING } from './types';
import type { TimelineResponse } from '~quent/types/TimelineResponse';
import { useTimelineChartColors } from './useTimelineChartColors';

const CONTROLLER_HEIGHT = 50;
const DEFAULT_NUM_BINS = 200;
const CONTROLLER_TOP_HEADROOM_RATIO = 0.2;

type TimelineControllerProps = {
  /** Start time in nanoseconds (bigint) */
  startTime: bigint;
  /** Duration in seconds */
  durationSeconds: number;
  /** Number of bins to match child timelines */
  numBins?: number;
  height?: number;
  /** Optional timeline data to render on the static display (e.g. overlay from root resource group) */
  timelineData?: TimelineResponse | null;
};

export function TimelineController({
  startTime,
  durationSeconds,
  numBins = DEFAULT_NUM_BINS,
  height = CONTROLLER_HEIGHT,
  timelineData,
}: TimelineControllerProps) {
  const colors = useTimelineChartColors();

  // Start time in milliseconds for formatting
  const startTimeMillis = useMemo(() => Number(startTime / 1_000_000n), [startTime]);

  const { timestamps, seriesData } = useMemo(() => {
    if (timelineData) {
      const { timestamps: ts, series } = buildBinnedTimelineSeries(timelineData, startTime);
      const entries = Object.entries(series);
      const values = entries.length > 0 ? entries[0][1].values : null;
      return { timestamps: ts, seriesData: values };
    } else {
      const binDurationMs = (durationSeconds * 1000) / numBins;
      const ts = Array.from({ length: numBins }, (_, i) =>
        Math.round(startTimeMillis + i * binDurationMs)
      );
      return { timestamps: ts, seriesData: null };
    }
  }, [timelineData, startTime, startTimeMillis, durationSeconds, numBins]);

  // Create series: zoom control first (drawn behind), then static display on top with higher z
  const hasSeriesData = useMemo(() => Boolean(seriesData && seriesData.length > 0), [seriesData]);

  const seriesOptions = useMemo(() => {
    const zoomControlSeries = {
      name: 'zoom-control',
      type: 'line',
      xAxisIndex: 1,
      data: Array(timestamps.length).fill(0),
      showSymbol: false,
      lineStyle: { width: 0 },
      areaStyle: { opacity: 0 },
      silent: true,
      z: 1,
    };
    const staticDisplaySeries = {
      name: 'static-display',
      type: 'line',
      xAxisIndex: 0,
      data: hasSeriesData ? seriesData : Array(timestamps.length).fill(0),
      showSymbol: false,
      lineStyle: { width: 1, color: colors.rollupTimelineColor },
      areaStyle: { color: withOpacity(colors.rollupTimelineColor, 0.8) },
      silent: true,
      step: 'middle',
      z: 1,
    };
    return [zoomControlSeries, staticDisplaySeries];
  }, [timestamps.length, seriesData, hasSeriesData, colors]);

  // Static x-axis (index 0): shows labels, ticks, gridlines - not affected by dataZoom
  const staticXAxisOptions = useMemo(
    () => ({
      data: timestamps,
      boundaryGap: false,
      type: 'category',
      show: true,
      axisLine: {
        show: true,
        lineStyle: { color: colors.gridBorderColor },
      },
      axisTick: { show: false },
      axisLabel: {
        show: true,
        fontSize: 10,
        color: colors.timelineMarkupColor,
        formatter: (value: number) => formatDuration(value - startTimeMillis),
      },
      splitLine: {
        show: true,
        lineStyle: {
          color: colors.gridBorderColor,
          type: 'solid',
        },
      },
    }),
    [timestamps, startTimeMillis, colors.timelineMarkupColor, colors.gridBorderColor]
  );

  // Hidden x-axis (index 1): controlled by dataZoom, no visible elements
  const zoomXAxisOptions = useMemo(
    () => ({
      data: timestamps,
      boundaryGap: false,
      type: 'category',
      show: false,
      axisLine: { show: false },
      axisTick: { show: false },
      axisLabel: { show: false },
      splitLine: { show: false },
    }),
    [timestamps]
  );

  const yAxisOptions = useMemo(() => {
    if (hasSeriesData) {
      return {
        type: 'value',
        show: false,
        min: 'dataMin',
        max: (value: { min: number; max: number }) => {
          const range = Math.max(value.max - value.min, 1);
          return value.max + range * CONTROLLER_TOP_HEADROOM_RATIO;
        },
        splitLine: { show: false },
      };
    }
    return {
      type: 'value',
      show: false,
      min: 0,
      max: 'datamax',
      splitLine: { show: false },
    };
  }, [hasSeriesData]);

  const gridOptions = useMemo(
    () => ({
      ...TIMELINE_SPACING,
      bottom: 20, // Make room for x-axis labels
      backgroundColor: colors.controllerGridBackgroundColor,
      borderWidth: 1,
      borderColor: colors.gridBorderColor,
      show: true,
    }),
    [colors.gridBorderColor, colors.controllerGridBackgroundColor]
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
          z: 10,
          xAxisIndex: [1], // Only control the hidden zoom axis (index 1), not the static display axis (index 0)
          realtime: true,
          top: 0,
          height: height - 24,
          brushSelect: true,
          handleStyle: {
            color: colors.dataZoomHandleColor,
            width: 2,
          },
          fillerColor: colors.dataZoomFillerColor,
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
          textStyle: {
            color: colors.dataZoomTextColor,
            opacity: 1,
            backgroundColor: colors.dataZoomTextBackgroundColor,
            padding: [2, 4],
            borderRadius: 2,
          },
          labelFormatter: (_: number, valueStr: string) => {
            const raw = Number(valueStr);
            const minTs = timestamps[0] ?? startTimeMillis;
            const maxTs = timestamps[timestamps.length - 1] ?? startTimeMillis;
            const clamped = Math.max(minTs, Math.min(maxTs, raw));
            return formatDuration(clamped - startTimeMillis);
          },
          emphasis: {
            handleStyle: {
              color: colors.dataZoomEmphasisHandleColor,
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
    colors,
    gridOptions,
    height,
    staticXAxisOptions,
    zoomXAxisOptions,
    yAxisOptions,
    seriesOptions,
    timestamps,
    startTimeMillis,
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
