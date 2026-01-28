import { useMemo } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import ReactECharts from 'echarts-for-react/lib/core';
import { echarts, connect } from '@/lib/echarts';
import type { EChartsOption } from '@/lib/echarts';
import { TooltipContent } from './TimelineTooltip';
import { getColorForKey, withOpacity } from '@/services/colors';
import { formatBytes } from '@/services/formatters';
import { TimelineSeries, DEFAULT_TIMELINE_HEIGHT } from './types';

export const CHART_GROUP = 'timeline-sync-group';
const TIMELINE_MARKUP_COLOR = '#808080';
const GRID_BORDER_COLOR = withOpacity(TIMELINE_MARKUP_COLOR, 0.2);

// Use 'any' for echarts instance type to avoid conflicts between echarts/core and full echarts types
const connectChart = (instance: { group: string }) => {
  instance.group = CHART_GROUP;
  connect(CHART_GROUP);
};

export function Timeline({
  startTime,
  series,
  timestamps,
  height = DEFAULT_TIMELINE_HEIGHT,
}: {
  startTime: bigint;
  series: TimelineSeries;
  timestamps: number[];
  height?: number;
}) {
  const seriesOptions = useMemo(() => {
    return (
      Object.entries(series)
        // TODO(joe): How should we sort series within the timeline?
        // Everything alphabetical RN to keep it consistent
        .sort((a, b) => a[0].localeCompare(b[0]))
        .map(([name, seriesData]) => {
          const color = getColorForKey(name);
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
            data: seriesData.values,
            lineStyle: { width: 0 },
            itemStyle: { color },
            areaStyle: { color: withOpacity(color, 0.9) },
            emphasis: {
              disabled: true, // Disable emphasis state
              focus: 'none', // Don't dim other series
            },
          };
        })
    );
  }, [series]);

  const yAxisOptions = useMemo(
    () => ({
      type: 'value',
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
        position: function (pt: number[]) {
          return [pt[0] + 25, '10%'];
        },
        formatter: function (hoveredSeries: unknown) {
          if (!Array.isArray(hoveredSeries) || hoveredSeries.length === 0) return '';
          const timestamp = Number(hoveredSeries[0].axisValue);
          const seriesValues = hoveredSeries.map(
            (p: { color: string; seriesName: string; value: number }) => ({
              color: p.color,
              name: p.seriesName,
              value: p.value,
            })
          );
          return renderToStaticMarkup(
            <TooltipContent timestamp={timestamp} series={seriesValues} startTime={startTime} />
          );
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
  }, [seriesOptions, yAxisOptions, xAxisOptions, gridOptions, startTime]);

  return (
    <ReactECharts
      echarts={echarts}
      option={eChartOptions}
      style={{ width: '100%', height: `${height}px` }}
      onChartReady={connectChart}
      notMerge
      lazyUpdate
    />
  );
}
