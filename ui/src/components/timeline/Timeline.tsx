import { useMemo } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import ReactECharts from 'echarts-for-react/lib/core';
import { echarts } from '@/lib/echarts';
import type { EChartsOption } from '@/lib/echarts';
import { TooltipContent } from './TimelineTooltip';
import { getColorForKey, withOpacity } from '@/services/colors';
import { formatBytes } from '@/services/formatters';
import { TimelineSeries, DEFAULT_TIMELINE_HEIGHT, TIMELINE_SPACING } from './types';
import { connectChart } from '@/lib/timeline.utils';
import { useTimelineChartColors } from './useTimelineChartColors';

export const CHART_GROUP = 'timeline-sync-group';

export function Timeline({
  startTime,
  series,
  timestamps,
  height = DEFAULT_TIMELINE_HEIGHT,
  colorKey,
  showTooltip = true,
}: {
  startTime: bigint;
  series: TimelineSeries;
  timestamps: number[];
  height?: number;
  colorKey?: string;
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
          // use colorKey for ResourceGroup coloring, otherwise use series name
          const color = getColorForKey(colorKey ?? name);
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
  }, [series, colorKey]);

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
        formatter: (value: number) => formatBytes(value, 0),
      },
    }),
    [gridBorderColor, timelineMarkupColor]
  );

  const xAxisOptions = useMemo(
    () => ({
      data: timestamps,
      boundaryGap: false,
      type: 'category',
      animation: false,
      interval: 0,
      show: true,
      axisLine: {
        show: true,
        onZero: true,
        lineStyle: { color: gridBorderColor },
      },
      axisTick: { show: false },
      axisLabel: { show: false },
      splitLine: {
        show: true,
        lineStyle: {
          color: gridBorderColor,
          type: 'solid' as const,
        },
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
    [timestamps, timelineMarkupColor, gridBorderColor]
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
