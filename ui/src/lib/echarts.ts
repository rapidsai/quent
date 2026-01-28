/**
 * Custom ECharts build with only the modules we need.
 * This significantly reduces bundle size (~1MB → ~300KB).
 *
 * To add new chart types or features, import and register them here.
 * See: https://echarts.apache.org/handbook/en/basics/import
 */

import * as echarts from 'echarts/core';
import type { ComposeOption, EChartsType } from 'echarts/core';

// Charts - only import what you use
import { LineChart } from 'echarts/charts';
import type { LineSeriesOption } from 'echarts/charts';

// Components - only import what you use
import {
  TitleComponent,
  TooltipComponent,
  GridComponent,
  DataZoomComponent,
  DataZoomInsideComponent,
  DataZoomSliderComponent,
} from 'echarts/components';
import type {
  TitleComponentOption,
  TooltipComponentOption,
  GridComponentOption,
  DataZoomComponentOption,
} from 'echarts/components';

// Renderer - use Canvas for better performance (SVG available if needed)
import { CanvasRenderer } from 'echarts/renderers';

// Register the required components
echarts.use([
  // Charts
  LineChart,
  // Components
  TitleComponent,
  TooltipComponent,
  GridComponent,
  DataZoomComponent,
  DataZoomInsideComponent,
  DataZoomSliderComponent,
  // Renderer
  CanvasRenderer,
]);

// Compose the option type from the components we use
export type EChartsOption = ComposeOption<
  | LineSeriesOption
  | TitleComponentOption
  | TooltipComponentOption
  | GridComponentOption
  | DataZoomComponentOption
>;

// Re-export echarts instance and types
export { echarts };
export type { EChartsType as ECharts };

// Re-export connect for chart synchronization
export const { connect } = echarts;
