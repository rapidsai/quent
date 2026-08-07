// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { CustomSeriesOption } from 'echarts/charts';
import type { GridComponentOption } from 'echarts/components';
import type { EChartsOption } from '../lib/echarts';
import { TIMELINE_SPACING, TIMELINE_X_AXIS_ANIMATION } from '../timeline/types';

export interface GanttDatum {
  value: [number, number, number];
}

export type GanttRenderItem = NonNullable<CustomSeriesOption['renderItem']>;
export type GanttSeriesCursor = CustomSeriesOption['cursor'];
export type GanttGridSpacing = Pick<GridComponentOption, 'left' | 'right' | 'top' | 'bottom'>;

interface BuildGanttOptionParams<T extends GanttDatum> {
  data: T[];
  durationSeconds: number;
  yAxisCategories: number[];
  seriesName: string;
  renderItem: GanttRenderItem;
  minZoomSpanPct: number;
  cursor?: GanttSeriesCursor;
  gridSpacing?: GanttGridSpacing;
}

export function buildGanttOption<T extends GanttDatum>({
  data,
  durationSeconds,
  yAxisCategories,
  seriesName,
  renderItem,
  minZoomSpanPct,
  cursor,
  gridSpacing = TIMELINE_SPACING,
}: BuildGanttOptionParams<T>): EChartsOption {
  return {
    animation: false,
    axisPointer: {
      link: [{ xAxisIndex: 'all' }],
    },
    grid: {
      ...gridSpacing,
      width: undefined,
      height: undefined,
    },
    xAxis: {
      type: 'value',
      min: 0,
      max: durationSeconds * 1_000,
      show: true,
      axisLabel: { show: false },
      axisPointer: {
        show: true,
        type: 'line',
        animation: false,
        label: { show: false },
      },
      ...TIMELINE_X_AXIS_ANIMATION,
    },
    yAxis: {
      type: 'category',
      data: yAxisCategories,
      inverse: true,
      axisLine: { show: false },
      axisLabel: { show: false },
      axisPointer: { show: false },
    },
    series: [
      {
        type: 'custom',
        name: seriesName,
        animation: false,
        cursor,
        data,
        renderItem: renderItem as never,
        coordinateSystem: 'cartesian2d',
        encode: { x: [0, 1], y: 2 },
      },
    ],
    dataZoom: [
      {
        type: 'slider',
        show: false,
        realtime: true,
        filterMode: 'none',
        xAxisIndex: [0],
        minSpan: minZoomSpanPct,
      },
      {
        type: 'inside',
        zoomLock: true,
        zoomOnMouseWheel: false,
        moveOnMouseWheel: false,
        throttle: 30,
        filterMode: 'none',
        xAxisIndex: [0],
      },
      {
        type: 'inside',
        zoomOnMouseWheel: 'shift',
        moveOnMouseMove: false,
        moveOnMouseWheel: false,
        throttle: 30,
        filterMode: 'none',
        xAxisIndex: [0],
        minSpan: minZoomSpanPct,
      },
    ],
  };
}
