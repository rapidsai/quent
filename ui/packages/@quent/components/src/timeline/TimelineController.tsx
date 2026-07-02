// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import EChartsReactCore from 'echarts-for-react/lib/core';
import { echarts } from '../lib/echarts';
import type { EChartsOption } from '../lib/echarts';
import type { EChartsInstance } from 'echarts-for-react';
import { useZoomRange } from '@quent/hooks';
import { formatDuration } from '@quent/utils';
import type { ZoomRange } from '@quent/utils';
import {
  buildBinnedTimelineSeries,
  getAdaptiveNumBins,
  getTimelineXAxisIntervalMs,
  MIN_ZOOM_WINDOW_S,
  nanosToMs,
  registerAxisPointerSync,
  unregisterAxisPointerSync,
} from '../lib/timeline.utils';
import { useChartConnect } from '../lib/useChartConnect';
import { TIMELINE_X_AXIS_ANIMATION, TIMELINE_SPACING } from './types';
import type { SingleTimelineResponse } from '@quent/utils';
import { useTimelineEchartsTheme } from './timelineEchartsTheme';
import type { PaletteTheme } from '@quent/utils';
import { Opts } from 'echarts-for-react/lib/types';

const CONTROLLER_HEIGHT = 50;
const CONTROLLER_TOP_HEADROOM_RATIO = 0.2;
const CONTROLLER_X_MIN_LABELS = 8;
/** Reserves space for the top-positioned xAxis labels. */
const CONTROLLER_GRID_TOP = 20;
// Balanced with CONTROLLER_GRID_TOP so the chart area is centered in the controller height.
const CONTROLLER_GRID_BOTTOM = 10;
// Shifts the datazoom slider up so it overlaps the label row instead of sitting flush with the chart.
const DATAZOOM_TOP_OFFSET = 5;

type TimelineControllerProps = {
  startTime: bigint;
  durationSeconds: number;
  height?: number;
  timelineData?: SingleTimelineResponse | null;
  onZoomChange?: (range: ZoomRange) => void;
  isDark: boolean;
};

/** Zoom controller bar with datazoom slider and optional background timeline data. */
export function TimelineController({
  startTime,
  durationSeconds,
  height = CONTROLLER_HEIGHT,
  timelineData,
  onZoomChange,
  isDark,
}: TimelineControllerProps) {
  const { themeName, controllerGridBackgroundColor } = useTimelineEchartsTheme(isDark);
  const paletteTheme: PaletteTheme = isDark ? 'dark' : 'light';

  const startTimeMillis = useMemo(() => nanosToMs(startTime), [startTime]);

  const { timestamps, seriesData } = useMemo(() => {
    if (timelineData) {
      const { timestamps: ts, series } = buildBinnedTimelineSeries(
        timelineData.data,
        timelineData.config,
        startTime,
        paletteTheme
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
  }, [timelineData, startTime, startTimeMillis, durationSeconds, paletteTheme]);

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
    // Color comes from the timeline theme's `rollupTimelineColor` palette entry.
    const staticDisplaySeries = {
      name: 'static-display',
      type: 'line',
      xAxisIndex: 0,
      data: toTimePoints(staticValues ?? []),
      symbol: 'none',
      lineStyle: { width: 1 },
      areaStyle: { opacity: 0.8 },
      silent: true,
      emphasis: { disabled: true },
      step: 'middle',
      ...TIMELINE_X_AXIS_ANIMATION,
      z: 1,
    };

    // staticDisplaySeries first so ECharts treats xAxis[0] (full range) as
    // the primary tooltip axis — synced times outside xAxis[1]'s zoom render.
    return [staticDisplaySeries, zoomControlSeries];
  }, [timestamps, hasSeriesData, seriesData]);

  const endTimeMillis = startTimeMillis + durationSeconds * 1000;

  const staticXAxisOptions = useMemo(() => {
    const interval = getTimelineXAxisIntervalMs(
      endTimeMillis - startTimeMillis,
      CONTROLLER_X_MIN_LABELS
    );

    return {
      boundaryGap: [0, 0] as [number, number],
      type: 'value',
      show: true,
      position: 'top',
      min: startTimeMillis,
      max: endTimeMillis,
      interval,
      // Re-enable ticks the shared theme disables; default `inside: false`
      // points them up toward the top-positioned labels.
      axisTick: { show: true, alignWithLabel: true },
      axisLabel: {
        hideOverlap: false,
        // Keeps min/max axis labels contained within the chart
        alignMinLabel: 'left',
        alignMaxLabel: 'right',
        formatter: (value: number) => {
          return formatDuration(Number(value) - startTimeMillis);
        },
      },
      splitLine: { show: true, lineStyle: { type: 'solid' } },
      axisPointer: {
        show: true,
        type: 'line',
        snap: false,
        label: { show: false },
        handle: { show: false },
      },
    };
  }, [startTimeMillis, endTimeMillis]);

  const zoomXAxisOptions = useMemo(
    () => ({
      boundaryGap: [0, 0] as [number, number],
      type: 'value',
      show: false,
      min: startTimeMillis,
      max: endTimeMillis,
      axisLine: { show: false },
      axisTick: { show: false },
      axisLabel: { show: false },
      splitLine: { show: false },
      // Suppress pointer on the dataZoom'd axis — only xAxis[0] draws the crosshair.
      axisPointer: { show: false },
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
      top: CONTROLLER_GRID_TOP,
      bottom: CONTROLLER_GRID_BOTTOM,
      backgroundColor: controllerGridBackgroundColor,
      outerBoundsMode: 'none',
    }),
    [controllerGridBackgroundColor]
  );

  const minZoomSpanPct = useMemo(() => {
    if (durationSeconds <= 0) return 0;
    return Math.min(100, (MIN_ZOOM_WINDOW_S / durationSeconds) * 100);
  }, [durationSeconds]);

  const eChartOptions: EChartsOption = useMemo(() => {
    return {
      tooltip: {
        show: true,
        showContent: false,
        trigger: 'axis',
        // Pin to xAxis[0] (full range) so synced times outside the dataZoom
        // window of xAxis[1] still resolve and render a crosshair.
        axisPointer: { axis: 'x', xAxisIndex: 0 },
      },
      // Link only the static axis so the dataZoom'd xAxis[1] doesn't draw a second crosshair.
      axisPointer: {
        link: [{ xAxisIndex: [0] }],
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
          minSpan: minZoomSpanPct,
          top: CONTROLLER_GRID_TOP - DATAZOOM_TOP_OFFSET,
          height: height - CONTROLLER_GRID_TOP - CONTROLLER_GRID_BOTTOM,
          brushSelect: true,
          // Built-in labels disabled — replaced by custom DOM chips below.
          showDetail: false,
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
          minSpan: minZoomSpanPct,
        },
      ],
      xAxis: [staticXAxisOptions, zoomXAxisOptions],
      yAxis: yAxisOptions,
      series: seriesOptions,
    } as EChartsOption;
  }, [
    gridOptions,
    height,
    minZoomSpanPct,
    staticXAxisOptions,
    zoomXAxisOptions,
    yAxisOptions,
    seriesOptions,
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

  const selfTriggeredRef = useRef(false);
  // Bumped on chart-ready so the restore effect re-runs when the instance is
  // recreated (e.g. theme change disposes and rebuilds the chart at 0–100%).
  const [readyTick, setReadyTick] = useState(0);

  const zoomRange = useZoomRange();

  const onChartReady = useCallback((instance: EChartsInstance) => {
    registerAxisPointerSync(instance);
    setReadyTick(t => t + 1);
  }, []);

  const { handleChartReady, instanceRef } = useChartConnect({
    durationSeconds,
    activateBrushSelect: true,
    onReady: onChartReady,
  });

  // Restore the persisted zoom on range change or instance (re)creation.
  useEffect(() => {
    if (readyTick === 0) return;
    if (selfTriggeredRef.current) {
      selfTriggeredRef.current = false;
      return;
    }
    const instance = instanceRef.current;
    if (!instance || durationSeconds === 0) return;

    const startPct = (zoomRange.start / durationSeconds) * 100;
    const endPct = (zoomRange.end / durationSeconds) * 100;

    // Mute our own dispatch so the echoed dataZoom event doesn't overwrite the atom.
    selfTriggeredRef.current = true;
    instance.dispatchAction({
      type: 'dataZoom',
      dataZoomIndex: 0,
      start: startPct,
      end: endPct,
    });
  }, [readyTick, zoomRange, durationSeconds, instanceRef]);

  useEffect(() => {
    return () => {
      if (instanceRef.current) {
        unregisterAxisPointerSync(instanceRef.current);
        instanceRef.current = null;
      }
    };
  }, [instanceRef]);

  const opts = useMemo(() => ({ renderer: 'svg' }) as Opts, []);

  return (
    <EChartsReactCore
      echarts={echarts}
      theme={themeName}
      option={eChartOptions}
      style={{ width: '100%', height: `${height}px` }}
      onChartReady={handleChartReady}
      onEvents={handleDataZoom}
      notMerge={false}
      lazyUpdate
      opts={opts}
      autoResize={false}
    />
  );
}
