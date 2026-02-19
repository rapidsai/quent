import { useMemo, useCallback } from 'react';
import ReactECharts from 'echarts-for-react/lib/core';
import { echarts } from '@/lib/echarts';
import type { EChartsOption } from '@/lib/echarts';
import { withOpacity } from '@/services/colors';
import { formatDuration } from '@/services/formatters';
import { buildBinnedTimelineSeries, getTimelineXAxisIntervalMs } from '@/lib/timeline.utils';
import { TIMELINE_X_AXIS_ANIMATION, TIMELINE_SPACING } from './types';
import type { TimelineResponse } from '~quent/types/TimelineResponse';
import { useTimelineChartColors } from './useTimelineChartColors';

const CONTROLLER_HEIGHT = 50;
const DEFAULT_NUM_BINS = 200;
const CONTROLLER_TOP_HEADROOM_RATIO = 0.2;
const CONTROLLER_X_MIN_LABELS = 8;

interface DataZoomPayload {
  start?: number;
  end?: number;
  batch?: Array<{ start?: number; end?: number }>;
}

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
  /** Callback fired when the user drags the zoom handles */
  onZoomChange?: (event: { start: number; end: number }) => void;
};

export function TimelineController({
  startTime,
  durationSeconds,
  numBins = DEFAULT_NUM_BINS,
  height = CONTROLLER_HEIGHT,
  timelineData,
  onZoomChange,
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

  const handleDataZoom = useCallback(
    (params: DataZoomPayload) => {
      if (!onZoomChange) return;
      const { start, end } =
        params.start !== undefined && params.end !== undefined ? params : (params.batch?.[0] ?? {});
      if (start !== undefined && end !== undefined) {
        onZoomChange({ start, end });
      }
    },
    [onZoomChange]
  );

  // Create series: zoom control first (drawn behind), then static display on top with higher z
  const hasSeriesData = useMemo(() => Boolean(seriesData && seriesData.length > 0), [seriesData]);

  const seriesOptions = useMemo(() => {
    const toTimePoints = (values: number[]) =>
      values.map((value, index) => [timestamps[index], value] as [number, number]);

    const zoomControlSeries = {
      name: 'zoom-control',
      type: 'line',
      xAxisIndex: 1,
      data: toTimePoints(Array(timestamps.length).fill(0)),
      showSymbol: false,
      lineStyle: { width: 0 },
      areaStyle: { opacity: 0 },
      silent: true,
      z: 1,
    };
    const staticValues: number[] | null = hasSeriesData
      ? seriesData
      : Array(timestamps.length).fill(0);
    const staticDisplaySeries = {
      name: 'static-display',
      type: 'line',
      xAxisIndex: 0,
      data: toTimePoints(staticValues ?? []),
      showSymbol: false,
      lineStyle: { width: 1, color: colors.rollupTimelineColor },
      areaStyle: { color: withOpacity(colors.rollupTimelineColor, 0.8) },
      silent: true,
      step: 'middle',
      ...TIMELINE_X_AXIS_ANIMATION,
      z: 1,
    };
    return [zoomControlSeries, staticDisplaySeries];
  }, [timestamps, hasSeriesData, seriesData, colors.rollupTimelineColor]);

  // Static x-axis (index 0): shows labels, ticks, gridlines - not affected by dataZoom
  const staticXAxisOptions = useMemo(() => {
    const minTs = timestamps[0] ?? startTimeMillis;
    const maxTs = timestamps[timestamps.length - 1] ?? startTimeMillis;
    const interval = getTimelineXAxisIntervalMs(maxTs - minTs, CONTROLLER_X_MIN_LABELS);

    return {
      boundaryGap: false,
      type: 'value',
      show: true,
      min: minTs,
      max: maxTs,
      interval,
      axisLine: {
        show: true,
        lineStyle: { color: colors.gridBorderColor },
      },
      axisTick: { show: true },
      axisLabel: {
        show: true,
        hideOverlap: false,
        fontSize: 10,
        color: colors.timelineMarkupColor,
        formatter: (value: number) => {
          return formatDuration(Number(value) - startTimeMillis);
        },
      },
      splitLine: {
        show: true,
        lineStyle: {
          color: colors.gridBorderColor,
          type: 'solid',
        },
      },
    };
  }, [timestamps, startTimeMillis, colors.timelineMarkupColor, colors.gridBorderColor]);

  // Hidden x-axis (index 1): controlled by dataZoom, no visible elements
  const zoomXAxisOptions = useMemo(() => {
    const minTs = timestamps[0] ?? startTimeMillis;
    const maxTs = timestamps[timestamps.length - 1] ?? startTimeMillis;
    const interval = getTimelineXAxisIntervalMs(maxTs - minTs, CONTROLLER_X_MIN_LABELS);

    return {
      boundaryGap: false,
      type: 'value',
      show: false,
      // These bound and spec the values passed around to the various formatters, etc
      min: minTs,
      max: maxTs,
      interval,
      axisLine: { show: false },
      axisTick: { show: false },
      axisLabel: { show: false },
      splitLine: { show: false },
    };
  }, [timestamps, startTimeMillis]);

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
          filterMode: 'none',
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
          labelFormatter: (tsMilliseconds: number) => {
            const raw = Number(tsMilliseconds);
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
          filterMode: 'none',
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
      onEvents={onZoomChange ? { dataZoom: handleDataZoom } : undefined}
      notMerge={false}
      lazyUpdate
      opts={{ renderer: 'canvas' }}
    />
  );
}
