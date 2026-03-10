import { useMemo, useCallback } from 'react';
import ReactECharts from 'echarts-for-react/lib/core';
import { echarts } from '@/lib/echarts';
import type { EChartsOption } from '@/lib/echarts';
import type { EChartsInstance } from 'echarts-for-react';
import { useAtomValue } from 'jotai';
import { connectChart } from '@/lib/timeline.utils';
import { CHART_GROUP } from '@/components/timeline/Timeline';
import { nanosToMs } from '@/lib/timeline.utils';
import { getAdaptiveNumBins } from '@/lib/timeline.utils';
import { useTimelineChartColors } from '@/components/timeline/useTimelineChartColors';
import { zoomRangeAtom } from '@/atoms/timeline';
import { getColorForKey, withOpacity } from '@/services/colors';
import type { OperatorActiveSpanEntry } from './types';
import { TIMELINE_SPACING } from '@/components/timeline/types';

const DEFAULT_HEIGHT = 160;

export interface OperatorHeatmapChartProps {
  operators: OperatorActiveSpanEntry[];
  startTime: bigint;
  durationSeconds: number;
  height?: number;
  /** Optional bucket width in ms; default derived from duration and adaptive bins */
  bucketMs?: number;
}

export function OperatorHeatmapChart({
  operators,
  startTime,
  height = DEFAULT_HEIGHT,
  bucketMs: bucketMsProp,
}: OperatorHeatmapChartProps) {
  const { gridBorderColor, gridBackgroundColor, timelineMarkupColor } = useTimelineChartColors();
  const zoomRange = useAtomValue(zoomRangeAtom);
  const startTimeMs = useMemo(() => nanosToMs(startTime), [startTime]);

  const visibleSpanMs = (zoomRange.end - zoomRange.start) * 1_000;
  const numBins = getAdaptiveNumBins();
  const bucketMs = useMemo(
    () => bucketMsProp ?? Math.max(1, visibleSpanMs / numBins),
    [bucketMsProp, visibleSpanMs, numBins]
  );

  const visibleStartMs = useMemo(
    () => startTimeMs + zoomRange.start * 1_000,
    [startTimeMs, zoomRange.start]
  );

  const heatmapData = useMemo(() => {
    const data: [number, number, number][] = [];
    for (let b = 0; b < numBins; b++) {
      const bucketStartMs = visibleStartMs + b * bucketMs;
      const bucketEndMs = bucketStartMs + bucketMs;
      for (const op of operators) {
        const overlaps = op.startMs < bucketEndMs && op.endMs > bucketStartMs;
        if (overlaps) {
          data.push([b, op.rowIndex, 1]);
        }
      }
    }
    return data;
  }, [operators, visibleStartMs, numBins, bucketMs]);

  const accentColor = useMemo(() => getColorForKey('operator'), []);
  const lowColor = useMemo(() => withOpacity(accentColor, 0.15), [accentColor]);

  const gridOptions = useMemo(
    () => ({
      ...TIMELINE_SPACING,
      top: TIMELINE_SPACING.top + 8,
      bottom: TIMELINE_SPACING.bottom + 8,
      left: TIMELINE_SPACING.left,
      right: TIMELINE_SPACING.right,
      backgroundColor: gridBackgroundColor,
      borderWidth: 1,
      borderColor: gridBorderColor,
      show: true,
    }),
    [gridBackgroundColor, gridBorderColor]
  );

  const xCategories = useMemo(
    () => Array.from({ length: numBins }, (_, i) => i),
    [numBins]
  );

  const yCategories = useMemo(() => operators.map(op => op.label), [operators]);

  const option: EChartsOption = useMemo(
    () => ({
      animation: false,
      tooltip: {
        trigger: 'item',
        formatter: (params: unknown) => {
          const p = params as { data: [number, number, number] };
          const [bucketIndex, operatorIndex] = p.data;
          const op = operators[operatorIndex];
          const bucketStartMs = visibleStartMs + bucketIndex * bucketMs;
          const timeSec = (bucketStartMs - visibleStartMs) / 1_000;
          return op ? `${op.label}<br/>~${timeSec.toFixed(1)}s` : '';
        },
      },
      grid: gridOptions,
      xAxis: {
        type: 'category',
        data: xCategories,
        show: true,
        axisLine: { show: true, lineStyle: { color: gridBorderColor } },
        axisTick: { show: false },
        axisLabel: {
          show: true,
          fontSize: 10,
          color: timelineMarkupColor,
          formatter: (_value: string, index: number) => {
            const ms = visibleStartMs + index * bucketMs - visibleStartMs;
            const sec = ms / 1_000;
            return sec >= 1 ? `${sec.toFixed(0)}s` : `${ms.toFixed(0)}ms`;
          },
        },
        splitLine: { show: false },
      },
      yAxis: {
        type: 'category',
        data: yCategories,
        inverse: true,
        show: true,
        axisLine: { show: false },
        axisTick: { show: false },
        splitLine: { show: false },
        axisLabel: {
          show: true,
          fontSize: 10,
          color: timelineMarkupColor,
          margin: 8,
        },
      },
      visualMap: {
        show: false,
        min: 0,
        max: 1,
        inRange: {
          color: [lowColor, accentColor],
        },
      },
      series: [
        {
          type: 'heatmap',
          name: 'operator-activity',
          data: heatmapData,
          coordinateSystem: 'cartesian2d',
          emphasis: {
            itemStyle: {
              borderColor: accentColor,
              borderWidth: 1,
            },
          },
        },
      ],
    }),
    [
      gridOptions,
      xCategories,
      yCategories,
      operators,
      heatmapData,
      visibleStartMs,
      bucketMs,
      lowColor,
      accentColor,
      gridBorderColor,
      timelineMarkupColor,
    ]
  );

  const handleChartReady = useCallback((instance: EChartsInstance) => {
    connectChart(instance, CHART_GROUP, false);
  }, []);

  if (operators.length === 0) {
    return (
      <div
        className="flex items-center justify-center text-muted-foreground text-sm"
        style={{ height }}
      >
        No operator active spans
      </div>
    );
  }

  return (
    <ReactECharts
      echarts={echarts}
      option={option}
      style={{ height }}
      onChartReady={handleChartReady}
      notMerge
    />
  );
}
