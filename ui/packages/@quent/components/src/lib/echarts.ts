// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

/**
 * Custom ECharts build — only SVGRenderer + LineChart + CustomChart + the components we actually use.
 * To add chart types or features, import and register them here.
 * See: https://echarts.apache.org/handbook/en/basics/import
 */

import * as echarts from 'echarts/core';
import type { ComposeOption, EChartsType } from 'echarts/core';

// Charts - only import what you use
import { LineChart, CustomChart } from 'echarts/charts';
import type { LineSeriesOption, CustomSeriesOption } from 'echarts/charts';

// Components - only import what you use
import {
  TooltipComponent,
  GridComponent,
  DataZoomComponent,
  DataZoomInsideComponent,
  DataZoomSliderComponent,
  MarkAreaComponent,
} from 'echarts/components';
import type {
  TooltipComponentOption,
  GridComponentOption,
  DataZoomComponentOption,
  MarkAreaComponentOption,
} from 'echarts/components';

import { SVGRenderer } from 'echarts/renderers';

echarts.use([
  LineChart,
  CustomChart,
  TooltipComponent,
  GridComponent,
  DataZoomComponent,
  DataZoomInsideComponent,
  DataZoomSliderComponent,
  MarkAreaComponent,
  SVGRenderer,
]);

export type EChartsOption = ComposeOption<
  | LineSeriesOption
  | CustomSeriesOption
  | TooltipComponentOption
  | GridComponentOption
  | DataZoomComponentOption
  | MarkAreaComponentOption
>;

// Re-export echarts instance and types
export { echarts };
export type { EChartsType as ECharts };

// Re-export connect and getInstanceByDom for chart synchronization
export const { connect, disconnect, getInstanceByDom } = echarts;
