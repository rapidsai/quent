import { useMemo } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import ReactECharts from 'echarts-for-react';
import type { EChartsOption, ECharts } from 'echarts';
import { connect, graphic } from 'echarts';
import { TooltipContent } from './TimelineTooltip';

export const CHART_GROUP = 'timeline-sync-group';

const connectChart = (instance: ECharts, chartGroup: string = CHART_GROUP) => {
  instance.group = chartGroup;
  connect(chartGroup);
};

// ECharts default color palette
const COLOR_PALETTE = [
  '#5470c6',
  '#91cc75',
  '#fac858',
  '#ee6666',
  '#73c0de',
  '#3ba272',
  '#fc8452',
  '#9a60b4',
  '#ea7ccc',
];

const epochFormatter = (value: number) => {
  return new Date(value / 1000).toLocaleString('en-US', {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
};

export function Timeline({
  series,
  timestamps,
  height = 125,
}: {
  series: Record<string, number[]>;
  timestamps: number[];
  height?: number;
}) {
  const seriesOptions = useMemo(() => {
    return Object.entries(series).map(([name, data], index) => {
      const color = COLOR_PALETTE[index % COLOR_PALETTE.length];
      const areaGradient = new graphic.LinearGradient(0, 0, 0, 1, [
        { offset: 0, color: color + 'CC' }, // Top: 80% opacity
        { offset: 1, color: color + '0D' }, // Bottom: 5% opacity
      ]);
      return {
        name,
        type: 'line',
        step: 'middle',
        symbol: 'circle',
        symbolSize: 6,
        // Shows on hover
        showSymbol: false,
        hoverAnimation: false,
        animation: false,
        cursor: 'default',
        data,
        lineStyle: { color, width: 2 },
        itemStyle: { color },
        areaStyle: { color: areaGradient },
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
    }),
    []
  );

  const xAxisOptions = useMemo(
    () => ({
      data: timestamps,
      boundaryGap: false,
      type: 'category',
      animation: false,
      axisLabel: {
        show: false,
        formatter: epochFormatter,
      },
      axisLine: { onZero: true },
    }),
    [timestamps]
  );

  const gridOptions = useMemo(
    () => ({
      left: 40,
      right: 10,
      top: 10,
      bottom: 30,
      backgroundColor: 'rgba(128, 128, 128, 0.1)',
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
          const date = new Date(timestamp / 1000);
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
          xAxisIndex: seriesOptions.map((_, i) => i),
        },
        {
          type: 'inside',
          realtime: true,
          start: 0,
          end: 100,
          xAxisIndex: seriesOptions.map((_, i) => i),
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
      style={{ height: `${height}px` }}
      onChartReady={connectChart}
      notMerge
      lazyUpdate
    />
  );
}
