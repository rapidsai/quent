import { useMemo } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import ReactECharts from 'echarts-for-react/lib/core';
import { echarts } from '@/lib/echarts';
import type { EChartsOption } from '@/lib/echarts';
import { TooltipContent } from './TimelineTooltip';
import { withOpacity } from '@/services/colors';
import { formatBytes } from '@/services/formatters';
import {
  TimelineSeries,
  DEFAULT_TIMELINE_HEIGHT,
  TIMELINE_SPACING,
  TIMELINE_X_AXIS_ANIMATION,
} from './types';
import { connectChart } from '@/lib/timeline.utils';
import { useTimelineChartColors } from './useTimelineChartColors';

export const CHART_GROUP = 'timeline-sync-group';

export function Timeline({
  startTime,
  series,
  timestamps,
  height = DEFAULT_TIMELINE_HEIGHT,
  showTooltip = true,
}: {
  startTime: bigint;
  series: TimelineSeries;
  timestamps: number[];
  height?: number;
  showTooltip?: boolean;
}) {
  const { timelineMarkupColor, gridBorderColor, gridBackgroundColor } = useTimelineChartColors();

  const seriesOptions = useMemo(() => {
    return (
      Object.entries(series)
        // TODO(joe): How should we sort series within the timeline?
        // Everything alphabetical RN to keep it consistent
        .sort((a, b) => a[0].localeCompare(b[0]))
        .map(([name, seriesData]) => {
          const color = seriesData.color;
          // use colorKey for ResourceGroup coloring, otherwise use series name
          return {
            name,
            type: 'line',
            stack: 'total', // Stack all series with the same stack name
            step: 'middle',
            symbol: 'circle',
            symbolSize: 4,
            // Shows on hover
            hoverAnimation: false,
            showSymbol: false,
            // Keep initial render snappy, animate subsequent data updates
            ...TIMELINE_X_AXIS_ANIMATION,
            cursor: 'default',
            data: seriesData.values.map((value, index) => [timestamps[index], value]),
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
  }, [series, timestamps]);

  const yAxisOptions = useMemo(
    () => ({
      type: 'value',
      splitNumber: 1,
      show: true,
      axisLine: {
        show: true,
        lineStyle: { color: gridBorderColor },
      },
      axisTick: { show: false },
      splitLine: { show: false },
      axisLabel: {
        show: true,
        margin: 8,
        fontSize: 10,
        color: timelineMarkupColor,
        // TODO(joe): This needs to be dynamic, not always bytes but looks nice for now
        formatter: (value: number) => {
          return formatBytes(value, 0);
        },
      },
    }),
    [gridBorderColor, timelineMarkupColor]
  );

  const xAxisOptions = useMemo(
    () => ({
      boundaryGap: false,
      type: 'time',
      animation: false,
      show: true,
      axisLine: {
        show: true,
        onZero: true,
        lineStyle: { color: gridBorderColor },
      },
      axisTick: { show: false },
      axisLabel: { show: false },
      splitLine: {
        show: false,
      },
      axisPointer: {
        show: true, // Always show the axis pointer line (synced across charts)
        type: 'line',
        label: { show: false }, // Don't show the x-axis value label
        lineStyle: {
          type: 'dashed',
          color: timelineMarkupColor,
        },
      },
    }),
    [timelineMarkupColor, gridBorderColor]
  );

  const gridOptions = useMemo(
    () => ({
      ...TIMELINE_SPACING,
      backgroundColor: gridBackgroundColor,
      borderWidth: 1,
      borderColor: gridBorderColor,
      show: true,
    }),
    [gridBorderColor, gridBackgroundColor]
  );

  const eChartOptions: EChartsOption = useMemo(() => {
    return {
      tooltip: {
        show: showTooltip,
        trigger: 'axis',
        transitionDuration: 0, // Disable tooltip animation
        // We will style the tooltip in the component
        backgroundColor: 'transparent',
        borderWidth: 0,
        padding: 0,
        textStyle: {},
        confine: true,
        appendToBody: true,
        formatter: function (hoveredSeries: unknown) {
          if (!Array.isArray(hoveredSeries) || hoveredSeries.length === 0) return '';
          const timestamp = Number(hoveredSeries[0].axisValue);
          const seriesValues = hoveredSeries.map(
            (p: { color: string; seriesName: string; data: number[] }) => {
              console.log(p.data);
              return {
                color: p.color,
                name: p.seriesName,
                // Data is [timestamp, value] for 'time' type x-axis, this changes
                // if xAxis is changed to 'value' type
                value: p.data[1],
              };
            }
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
        link: [{ xAxisIndex: 'all' }],
      },
      grid: gridOptions,
      toolbox: {
        show: false, // Hide the toolbox UI, but feature is activated via dispatchAction
        feature: {
          dataZoom: {
            yAxisIndex: 'none', // Only zoom x-axis
          },
        },
      },
      dataZoom: [
        {
          type: 'slider',
          show: false,
          realtime: true,
          xAxisIndex: 'all',
          filterMode: 'none',
        },
      ],
      xAxis: xAxisOptions,
      yAxis: yAxisOptions,
      series: seriesOptions,
    } as EChartsOption;
  }, [seriesOptions, yAxisOptions, xAxisOptions, gridOptions, startTime, showTooltip]);

  return (
    <ReactECharts
      echarts={echarts}
      option={eChartOptions}
      style={{ width: '100%', height: `${height}px` }}
      onChartReady={connectChart}
      notMerge={false}
      lazyUpdate
    />
  );
}
