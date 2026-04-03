// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useMemo, useRef } from 'react';
import ReactECharts from 'echarts-for-react/lib/core';
import { echarts } from '@/lib/echarts';
import type { EChartsOption } from '@/lib/echarts';
import type { EChartsInstance } from 'echarts-for-react';
import { useAtomValue } from 'jotai';
import { withOpacity } from '@/services/colors';
import { formatDuration } from '@/services/formatters';
import {
  buildBinnedTimelineSeries,
  connectChart,
  getAdaptiveNumBins,
  getTimelineXAxisIntervalMs,
  nanosToMs,
  registerAxisPointerSync,
  unregisterAxisPointerSync,
} from '@/lib/timeline.utils';
import { TIMELINE_X_AXIS_ANIMATION, TIMELINE_SPACING } from './types';
import type { SingleTimelineResponse } from '~quent/types/SingleTimelineResponse';
import { useTimelineChartColors, TIMELINE_MONO_FONT } from './useTimelineChartColors';
import { zoomRangeAtom } from '@/atoms/timeline';

const CONTROLLER_HEIGHT = 50;
const CONTROLLER_TOP_HEADROOM_RATIO = 0.2;
const CONTROLLER_X_MIN_LABELS = 8;

export interface ZoomRange {
  start: number;
  end: number;
}

type TimelineControllerProps = {
  /** Start time in nanoseconds (bigint) */
  startTime: bigint;
  /** Duration in seconds */
  durationSeconds: number;
  height?: number;
  /** Optional timeline data to render on the static display (e.g. overlay from root resource group) */
  timelineData?: SingleTimelineResponse | null;
  /** Called when the zoom/pan range changes, with start/end in seconds */
  onZoomChange?: (range: ZoomRange) => void;
};

export function TimelineController({
  startTime,
  durationSeconds,
  height = CONTROLLER_HEIGHT,
  timelineData,
  onZoomChange,
}: TimelineControllerProps) {
  const colors = useTimelineChartColors();

  const startTimeMillis = useMemo(() => nanosToMs(startTime), [startTime]);

  const { timestamps, seriesData } = useMemo(() => {
    if (timelineData) {
      const { timestamps: ts, series } = buildBinnedTimelineSeries(
        timelineData.data,
        timelineData.config,
        startTime
      );
      const entries = Object.entries(series);
      const values = entries.length > 0 ? entries[0][1].values : null;
      return { timestamps: ts, seriesData: values };
    } else {
      const numBins = getAdaptiveNumBins();
      const binDurationMs = (durationSeconds * 1000) / numBins;
      const ts = Array.from({ length: numBins }, (_, i) => startTimeMillis + i * binDurationMs);
      return { timestamps: ts, seriesData: null };
    }
  }, [timelineData, startTime, startTimeMillis, durationSeconds]);

  const hasSeriesData = useMemo(() => Boolean(seriesData && seriesData.length > 0), [seriesData]);

  const seriesOptions = useMemo(() => {
    const toTimePoints = (values: number[]) =>
      values.map((value, index) => [timestamps[index], value] as [number, number]);

    const zoomControlSeries = {
      name: 'zoom-control',
      type: 'line',
      xAxisIndex: 1,
      data: toTimePoints(Array(timestamps.length).fill(0)),
      symbol: 'none',
      lineStyle: { width: 0 },
      areaStyle: { opacity: 0 },
      silent: true,
      emphasis: { disabled: true },
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
      symbol: 'none',
      lineStyle: { width: 1, color: colors.rollupTimelineColor },
      areaStyle: { color: withOpacity(colors.rollupTimelineColor, 0.8) },
      silent: true,
      emphasis: { disabled: true },
      step: 'middle',
      ...TIMELINE_X_AXIS_ANIMATION,
      z: 1,
    };

    return [zoomControlSeries, staticDisplaySeries];
  }, [timestamps, hasSeriesData, seriesData, colors.rollupTimelineColor]);

  const endTimeMillis = startTimeMillis + durationSeconds * 1000;

  const staticXAxisOptions = useMemo(() => {
    const interval = getTimelineXAxisIntervalMs(
      endTimeMillis - startTimeMillis,
      CONTROLLER_X_MIN_LABELS
    );

    return {
      boundaryGap: false,
      type: 'value',
      show: true,
      min: startTimeMillis,
      max: endTimeMillis,
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
        fontFamily: TIMELINE_MONO_FONT,
        formatter: (value: number) => {
          return formatDuration(Number(value) - startTimeMillis, 0);
        },
      },
      splitLine: {
        show: true,
        lineStyle: {
          color: colors.gridBorderColor,
          type: 'solid',
        },
      },
      axisPointer: {
        show: true,
        type: 'line',
        snap: false,
        label: { show: false },
        handle: { show: false },
        lineStyle: {
          type: 'dashed',
          color: colors.timelineMarkupColor,
        },
      },
    };
  }, [startTimeMillis, endTimeMillis, colors.timelineMarkupColor, colors.gridBorderColor]);

  const zoomXAxisOptions = useMemo(
    () => ({
      boundaryGap: false,
      type: 'value',
      show: false,
      min: startTimeMillis,
      max: endTimeMillis,
      axisLine: { show: false },
      axisTick: { show: false },
      axisLabel: { show: false },
      splitLine: { show: false },
    }),
    [startTimeMillis, endTimeMillis]
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
      bottom: 20,
      backgroundColor: colors.controllerGridBackgroundColor,
      borderWidth: 1,
      borderColor: colors.gridBorderColor,
      show: true,
    }),
    [colors.gridBorderColor, colors.controllerGridBackgroundColor]
  );

  const eChartOptions: EChartsOption = useMemo(() => {
    return {
      tooltip: { show: true, showContent: false, trigger: 'axis' },
      axisPointer: {
        link: [{ xAxisIndex: 'all' }],
      },
      grid: gridOptions,
      dataZoom: [
        {
          type: 'slider',
          show: true,
          z: 10,
          xAxisIndex: [1],
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
            fontFamily: TIMELINE_MONO_FONT,
          },
          labelFormatter: (tsMilliseconds: number) => {
            return formatDuration(Number(tsMilliseconds) - startTimeMillis, 0);
          },
          emphasis: {
            handleStyle: {
              color: colors.dataZoomEmphasisHandleColor,
            },
          },
        },
        {
          type: 'inside',
          xAxisIndex: [1],
          realtime: true,
          filterMode: 'none',
          zoomLock: true,
          zoomOnMouseWheel: false,
          moveOnMouseMove: false,
        },
        {
          type: 'inside',
          xAxisIndex: [1],
          realtime: true,
          filterMode: 'none',
          zoomOnMouseWheel: true,
          moveOnMouseMove: false,
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

  const handleDataZoom = useMemo(() => {
    if (!onZoomChange) return undefined;
    return {
      dataZoom: (params: {
        start?: number;
        end?: number;
        batch?: Array<{ start?: number; end?: number }>;
      }) => {
        let start: number | undefined;
        let end: number | undefined;
        if (params.start !== undefined && params.end !== undefined) {
          start = params.start;
          end = params.end;
        } else if (params.batch?.[0]) {
          start = params.batch[0].start;
          end = params.batch[0].end;
        }
        if (start !== undefined && end !== undefined) {
          selfTriggeredRef.current = true;
          onZoomChange({
            start: (start / 100) * durationSeconds,
            end: (end / 100) * durationSeconds,
          });
        }
      },
    };
  }, [onZoomChange, durationSeconds]);

  const instanceRef = useRef<EChartsInstance | null>(null);
  const selfTriggeredRef = useRef(false);

  const zoomRange = useAtomValue(zoomRangeAtom);

  useEffect(() => {
    if (selfTriggeredRef.current) {
      selfTriggeredRef.current = false;
      return;
    }
    const instance = instanceRef.current;
    if (!instance || durationSeconds === 0) return;

    const startPct = (zoomRange.start / durationSeconds) * 100;
    const endPct = (zoomRange.end / durationSeconds) * 100;

    instance.dispatchAction({
      type: 'dataZoom',
      dataZoomIndex: 0,
      start: startPct,
      end: endPct,
    });
  }, [zoomRange, durationSeconds]);

  const handleChartReady = useCallback((instance: EChartsInstance) => {
    instanceRef.current = instance;
    connectChart(instance);
    registerAxisPointerSync(instance, 0);
  }, []);

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
      onEvents={handleDataZoom}
      notMerge={false}
      lazyUpdate
      opts={{ renderer: 'canvas' }}
    />
  );
}
