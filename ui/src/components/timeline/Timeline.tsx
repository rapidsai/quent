import { useMemo } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import ReactECharts from 'echarts-for-react';
import type { EChartsOption, ECharts } from 'echarts';
import { connect } from 'echarts';
import { TooltipContent } from './TimelineTooltip';
import { getColorByIndex, withOpacity } from '@/services/colors';
import { formatBytes } from '@/services/formatters';

export const CHART_GROUP = 'timeline-sync-group';

const connectChart = (instance: ECharts, chartGroup: string = CHART_GROUP) => {
  instance.group = chartGroup;
  connect(chartGroup);
};
export const DEFAULT_TIMELINE_HEIGHT = 100;
const TIMELINE_MARKUP_COLOR = '#808080';
const GRID_BORDER_COLOR = withOpacity(TIMELINE_MARKUP_COLOR, 0.2);

export function Timeline({
  series,
  timestamps,
  height = DEFAULT_TIMELINE_HEIGHT,
}: {
  series: Record<string, number[]>;
  timestamps: number[];
  height?: number;
}) {
  const seriesOptions = useMemo(() => {
    return Object.entries(series).map(([name, data], index) => {
      const color = getColorByIndex(index);
      return {
        name,
        type: 'line',
        stack: 'total', // Stack all series with the same stack name
        step: 'middle',
        symbol: 'circle',
        symbolSize: 4,
        // Shows on hover
        showSymbol: false,
        hoverAnimation: false,
        animation: false,
        cursor: 'default',
        data,
        lineStyle: { width: 0 },
        itemStyle: { color },
        areaStyle: { color: withOpacity(color, 0.9) },
        emphasis: {
          disabled: true, // Disable emphasis state
          focus: 'none', // Don't dim other series
        },
      };
    });
  }, [series]);

  const yAxisOptions = useMemo(
    () => ({
      type: 'value',
      boundaryGap: false,
      splitNumber: 1,
      show: true,
      axisLine: {
        show: true,
        lineStyle: { color: GRID_BORDER_COLOR },
      },
      axisTick: { show: false },
      splitLine: { show: false },
      axisLabel: {
        show: true,
        margin: 8,
        fontSize: 10,
        color: TIMELINE_MARKUP_COLOR,
        // TODO(joe): This needs to be dynamic, not always bytes but looks nice for now
        formatter: (value: number) => formatBytes(value, 0),
      },
    }),
    []
  );

  const xAxisOptions = useMemo(
    () => ({
      data: timestamps,
      boundaryGap: false,
      type: 'category',
      animation: false,
      show: true,
      axisLine: {
        show: true,
        onZero: true,
        lineStyle: { color: GRID_BORDER_COLOR },
      },
      axisTick: { show: false },
      axisLabel: { show: false },
    }),
    [timestamps]
  );

  const gridOptions = useMemo(
    () => ({
      left: 35,
      right: 2,
      top: 10,
      bottom: 10,
      backgroundColor: withOpacity(TIMELINE_MARKUP_COLOR, 0.1),
      borderWidth: 1,
      borderColor: GRID_BORDER_COLOR,
      show: true,
    }),
    []
  );

  const eChartOptions: EChartsOption = useMemo(() => {
    return {
      tooltip: {
        trigger: 'axis',
        // We will style the tooltip in the component
        backgroundColor: 'transparent',
        borderWidth: 0,
        padding: 0,
        textStyle: {},
        position: function (pt) {
          return [pt[0] + 25, '10%'];
        },
        formatter: function (params: unknown) {
          if (!Array.isArray(params) || params.length === 0) return '';
          const timestamp = Number(params[0].axisValue);
          const date = new Date(timestamp);
          const series = params.map((p: { color: string; seriesName: string; value: number }) => ({
            color: p.color,
            name: p.seriesName,
            value: p.value,
          }));
          return renderToStaticMarkup(<TooltipContent date={date} series={series} />);
        },
      },
      title: {
        left: 'center',
      },
      axisPointer: {
        link: [
          {
            xAxisIndex: 'all',
          },
        ],
      },
      grid: gridOptions,
      dataZoom: [
        {
          show: false,
          realtime: true,
          start: 0,
          end: 100,
          xAxisIndex: 'all',
        },
        {
          type: 'inside',
          realtime: true,
          start: 0,
          end: 100,
          xAxisIndex: 'all',
        },
      ],
      xAxis: xAxisOptions,
      yAxis: yAxisOptions,
      series: seriesOptions,
    } as EChartsOption;
  }, [seriesOptions, yAxisOptions, xAxisOptions, gridOptions]);

  return (
    <ReactECharts
      option={eChartOptions}
      style={{ width: '100%', height: `${height}px` }}
      onChartReady={connectChart}
      notMerge
      lazyUpdate
    />
  );
}
