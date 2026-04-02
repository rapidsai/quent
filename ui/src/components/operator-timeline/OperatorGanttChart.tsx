import { useCallback, useEffect, useLayoutEffect, useMemo, useRef } from 'react';
import ReactECharts from 'echarts-for-react/lib/core';

import type { EChartsOption } from '@/lib/echarts';
import type { EChartsInstance } from 'echarts-for-react';
import { useAtomValue, useSetAtom, useStore } from 'jotai';
import {
  nanosToMs,
  connectChart,
  registerAxisPointerSync,
  unregisterAxisPointerSync,
} from '@/lib/timeline.utils';
import { echarts } from '@/lib/echarts';
import { CHART_GROUP } from '@/components/timeline/Timeline';
import { useTimelineChartColors } from '@/components/timeline/useTimelineChartColors';
import { zoomRangeAtom } from '@/atoms/timeline';
import { selectedNodeIdsAtom, selectedOperatorLabelAtom, selectedPlanIdAtom } from '@/atoms/dag';
import { withOpacity } from '@/services/colors';
import type { OperatorActiveSpanEntry } from './types';
import { clipRectByRect } from './utils';
import { TIMELINE_SPACING, TIMELINE_X_AXIS_ANIMATION } from '@/components/timeline/types';

const DEFAULT_HEIGHT = 75;
const MAX_VISIBLE_ROWS = 10;
const BAR_FONT_SIZE = 10;
const BAR_HEIGHT = 16;

/** Border colors aligned with QueryPlanNode (Tailwind palette). Fill = stroke at ~15% opacity. */
// TODO(joe): Temporary, use @cmatzenbach colors once in.
const OPERATOR_COLORS: Record<string, string> = {
  source: '#3b82f6',
  scan: '#3b82f6',
  filesystemscan: '#3b82f6',
  join: '#a855f7',
  joinlocal: '#a855f7',
  joinpartition: '#a855f7',
  aggregate: '#22c55e',
  exchange: '#f97316',
  output: '#ef4444',
  stage: '#4f46e5',
  local: '#f59e0b',
  project: '#14b8a6',
  filter: '#06b6d4',
  sort: '#8b5cf6',
  limit: '#ec4899',
  union: '#10b981',
  other: '#6b7280',
};

function getOperatorBarColors(typeName: string | undefined): { fill: string; stroke: string } {
  const key = typeName?.toLowerCase().replace(/\s+/g, '') ?? 'other';
  const stroke = OPERATOR_COLORS[key] ?? OPERATOR_COLORS.other;
  return { stroke, fill: withOpacity(stroke, 0.15) };
}

export interface OperatorGanttChartProps {
  operators: OperatorActiveSpanEntry[];
  startTime: bigint;
  durationSeconds: number;
  height?: number;
}

export function OperatorGanttChart({
  operators,
  startTime,
  durationSeconds,
  height = DEFAULT_HEIGHT,
}: OperatorGanttChartProps) {
  const store = useStore();
  const setSelectedNodeIds = useSetAtom(selectedNodeIdsAtom);
  const setSelectedOperatorLabel = useSetAtom(selectedOperatorLabelAtom);
  const setSelectedPlanId = useSetAtom(selectedPlanIdAtom);
  const { gridBorderColor, gridBackgroundColor, timelineMarkupColor, textColor } =
    useTimelineChartColors();
  const barLabelTextColor = textColor;
  const selectedNodeIds = useAtomValue(selectedNodeIdsAtom);
  const startTimeMs = useMemo(() => nanosToMs(startTime), [startTime]);
  const xAxisMax = useMemo(
    () => startTimeMs + durationSeconds * 1_000,
    [startTimeMs, durationSeconds]
  );

  const { yAxisCategories, rowCount } = useMemo(() => {
    if (operators.length === 0) return { yAxisCategories: [] as number[], rowCount: 0 };
    const maxRow = Math.max(...operators.map(op => op.rowIndex));
    return {
      yAxisCategories: Array.from({ length: maxRow + 1 }, (_, i) => i),
      rowCount: maxRow + 1,
    };
  }, [operators]);

  const customSeriesData = useMemo(
    () =>
      operators.map(op => ({
        value: [op.startMs, op.endMs, op.rowIndex] as [number, number, number],
        name: op.label,
      })),
    [operators]
  );
  const showYScroll = rowCount > MAX_VISIBLE_ROWS;
  const yAxisZoomEnd = showYScroll ? (MAX_VISIBLE_ROWS / rowCount) * 100 : 100;

  const renderItem = useCallback(
    (
      params: {
        dataIndexInside: number;
        coordSys: { x: number; y: number; width: number; height: number };
      },
      api: {
        coord: (value: number[]) => number[];
        size: (value: number[]) => number[];
        value: (dim: number) => number;
      }
    ) => {
      const startMs = api.value(0) as number;
      const endMs = api.value(1) as number;
      const rowIndex = api.value(2) as number;
      if (endMs <= startMs) return null;
      const startPoint = api.coord([startMs, rowIndex]);
      const endPoint = api.coord([endMs, rowIndex]);

      // Full band height
      const barHeight = Math.max(BAR_FONT_SIZE + 4, BAR_HEIGHT);
      const y = startPoint[1] - barHeight / 2;
      const width = Math.max(1, endPoint[0] - startPoint[0]);

      // Clips boxes to the chart container
      const coord = params.coordSys as { x?: number; y?: number; width?: number; height?: number };
      const clipBound =
        typeof coord.width === 'number' && typeof coord.height === 'number'
          ? {
              x: coord.x ?? 0,
              y: coord.y ?? 0,
              width: coord.width,
              height: coord.height,
            }
          : null;
      const rectShape = {
        x: startPoint[0],
        y,
        width,
        height: barHeight,
      };
      const clippedShape = clipBound ? clipRectByRect(rectShape, clipBound) : rectShape;
      if (!clippedShape) return null;

      const op = operators[params.dataIndexInside];
      const barLabel =
        op?.typeName && op.typeName !== op.label
          ? `${op.typeName}: ${op.label}`
          : (op?.label ?? '');
      const { fill, stroke } = getOperatorBarColors(op?.typeName);
      const hasSelection = selectedNodeIds.size > 0;
      const isSelected = op != null && selectedNodeIds.has(op.operatorId);
      const opacity = hasSelection && !isSelected ? 0.35 : 1;

      const rect = {
        type: 'rect' as const,
        transition: [] as string[],
        shape: { ...clippedShape, r: 2 },
        style: {
          fill,
          stroke,
          lineWidth: 1,
          opacity,
        },
      };

      const textX = clippedShape.x + 6;
      const textY = clippedShape.y + clippedShape.height / 2;

      const text = {
        type: 'text' as const,
        transition: [] as string[],
        style: {
          text: barLabel,
          x: textX,
          y: textY,
          textVerticalAlign: 'middle' as const,
          fontSize: BAR_FONT_SIZE,
          fill: barLabelTextColor,
          overflow: 'truncate' as const,
          width: Math.max(0, clippedShape.width - 12),
          opacity,
        },
      };

      return {
        type: 'group' as const,
        children: [rect, text],
      };
    },
    [operators, barLabelTextColor, selectedNodeIds]
  );

  const gridOptions = useMemo(
    () => ({
      ...TIMELINE_SPACING,
      top: 0,
      bottom: 0,
      left: TIMELINE_SPACING.left,
      right: showYScroll ? TIMELINE_SPACING.right : TIMELINE_SPACING.right,
      width: undefined as number | undefined,
      height: undefined as number | undefined,
      backgroundColor: gridBackgroundColor,
      borderWidth: 1,
      borderColor: gridBorderColor,
      show: true,
    }),
    [gridBackgroundColor, gridBorderColor, showYScroll]
  );

  const option: EChartsOption = useMemo(
    () => ({
      animation: false,
      tooltip: { show: false },
      axisPointer: {
        link: [{ xAxisIndex: 'all' }],
      },
      grid: gridOptions,
      xAxis: {
        type: 'time',
        min: startTimeMs,
        max: xAxisMax,
        show: true,
        axisLine: { show: true, lineStyle: { color: gridBorderColor } },
        axisTick: { show: false },
        axisLabel: { show: false },
        splitLine: { show: false },
        axisPointer: {
          show: true,
          type: 'line',
          animation: false,
          label: { show: false },
          lineStyle: {
            type: 'dashed',
            color: timelineMarkupColor,
          },
        },
        ...TIMELINE_X_AXIS_ANIMATION,
      },
      yAxis: {
        type: 'category',
        data: yAxisCategories,
        inverse: true,
        axisLine: { show: false },
        axisTick: { show: false },
        axisLabel: { show: false },
        splitLine: { show: false },
        axisPointer: { show: false },
      },
      series: [
        {
          type: 'custom',
          name: 'operator-span',
          animation: false,
          cursor: 'pointer',
          data: customSeriesData,
          renderItem: renderItem as never,
          coordinateSystem: 'cartesian2d',
        },
      ],
      dataZoom: [
        {
          type: 'slider',
          show: false,
          realtime: true,
          filterMode: 'none',
          xAxisIndex: [0],
        },
        {
          type: 'inside',
          zoomLock: true,
          zoomOnMouseWheel: false,
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
        },
        ...(showYScroll
          ? [
              {
                type: 'inside' as const,
                yAxisIndex: [0],
                start: 0,
                end: yAxisZoomEnd,
                zoomLock: true,
                zoomOnMouseWheel: false,
                moveOnMouseMove: true,
                moveOnMouseWheel: true,
                throttle: 30,
                filterMode: 'none' as const,
              },
            ]
          : []),
      ],
    }),
    [
      gridOptions,
      startTimeMs,
      xAxisMax,
      gridBorderColor,
      timelineMarkupColor,
      yAxisCategories,
      customSeriesData,
      renderItem,
      showYScroll,
      yAxisZoomEnd,
    ]
  );

  const instanceRef = useRef<EChartsInstance | null>(null);

  const handleClick = useMemo(
    () => ({
      click: (params: { dataIndex: number; seriesName?: string }) => {
        if (params.seriesName !== 'operator-span') return;
        const op = operators[params.dataIndex];
        if (!op) return;
        if (selectedNodeIds.has(op.operatorId)) {
          setSelectedNodeIds(new Set());
          setSelectedOperatorLabel(null);
        } else {
          setSelectedNodeIds(new Set([op.operatorId]));
          setSelectedOperatorLabel(op.label);
          if (op.planId) {
            setSelectedPlanId(op.planId);
          }
        }
      },
    }),
    [operators, selectedNodeIds, setSelectedNodeIds, setSelectedOperatorLabel, setSelectedPlanId]
  );

  const handleChartReady = useCallback((instance: EChartsInstance) => {
    instanceRef.current = instance;
    // Join timeline-sync-group for frame-rate-level x-axis zoom sync via ECharts connect().
    // The y-axis dataZoom (index 3, when present) has a unique component ID and does not
    // propagate to resource timelines that have no matching component.
    connectChart(instance, CHART_GROUP, false);
    registerAxisPointerSync(instance, 0, { receiveShowTip: false });
    const dom = instance.getDom();
    dom.addEventListener(
      'wheel',
      (e: WheelEvent) => {
        if (!e.shiftKey) e.preventDefault();
      },
      { capture: true, passive: false }
    );
  }, []);

  // After a notMerge chart rebuild (operators change or selection change both cause one),
  // re-apply the current zoom synchronously — useLayoutEffect runs in the same browser frame
  // as ReactECharts' componentDidUpdate so there is no visible zoom-reset flash.
  useLayoutEffect(() => {
    const instance = instanceRef.current;
    if (!instance) return;
    const range = store.get(zoomRangeAtom);
    const dur = durationSeconds;
    if (dur <= 0) return;
    instance.dispatchAction({
      type: 'dataZoom',
      dataZoomIndex: 0,
      start: (range.start / dur) * 100,
      end: (range.end / dur) * 100,
    });
  }, [operators, selectedNodeIds, store, durationSeconds]);

  useEffect(() => {
    return () => {
      if (instanceRef.current) {
        unregisterAxisPointerSync(instanceRef.current);
        instanceRef.current = null;
      }
    };
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
      onEvents={handleClick}
      notMerge
    />
  );
}
