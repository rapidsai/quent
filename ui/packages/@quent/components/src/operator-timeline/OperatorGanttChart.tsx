// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useMemo, useRef } from 'react';
import ReactEChartsComponent from 'echarts-for-react';

import type { EChartsOption } from '../lib/echarts';
import type { EChartsInstance } from 'echarts-for-react';
import type { CustomSeriesOption } from 'echarts/charts';
import {
  nanosToMs,
  connectChart,
  registerAxisPointerSync,
  unregisterAxisPointerSync,
} from '../lib/timeline.utils';
import { echarts } from '../lib/echarts';
import { CHART_GROUP } from '../timeline/Timeline';
import { useTimelineEchartsTheme } from '../timeline/timelineEchartsTheme';
import {
  useSelectedNodeIds,
  useSetSelectedNodeIds,
  useSetSelectedOperatorLabel,
  useSetSelectedPlanId,
  useNodeColoringValue,
  useNodeColorPalette,
} from '@quent/hooks';
import { continuousColor, withOpacity } from '@quent/utils';
import {
  OPERATION_TYPE_COLORS,
  DEFAULT_OPERATION_COLOR,
} from '../services/query-plan/operationTypes';
import type { OperatorActiveSpanEntry } from './types';
import { clipRectByRect } from './utils';
import { TIMELINE_SPACING, TIMELINE_X_AXIS_ANIMATION } from '../timeline/types';

const DEFAULT_HEIGHT = 75;
const MAX_VISIBLE_ROWS = 10;
const BAR_FONT_SIZE = 10;
const BAR_HEIGHT = 16;

function getOperatorBarColors(typeName: string | undefined): { fill: string; stroke: string } {
  const key = typeName?.toLowerCase().replace(/\s+/g, '') ?? 'other';
  const stroke = OPERATION_TYPE_COLORS[key] ?? DEFAULT_OPERATION_COLOR;
  return { stroke, fill: withOpacity(stroke, 0.15) };
}

export interface OperatorGanttChartProps {
  operators: OperatorActiveSpanEntry[];
  startTime: bigint;
  durationSeconds: number;
  height?: number;
  /** Whether dark mode is active. Passed explicitly to decouple from ThemeContext. */
  isDark: boolean;
}

export function OperatorGanttChart({
  operators,
  startTime,
  durationSeconds,
  height = DEFAULT_HEIGHT,
  isDark,
}: OperatorGanttChartProps) {
  const setSelectedNodeIds = useSetSelectedNodeIds();
  const setSelectedOperatorLabel = useSetSelectedOperatorLabel();
  const setSelectedPlanId = useSetSelectedPlanId();
  const { themeName, textColor } = useTimelineEchartsTheme(isDark);
  const nodeColoring = useNodeColoringValue();
  const [nodePalette] = useNodeColorPalette();
  const barLabelTextColor = textColor;
  const selectedNodeIds = useSelectedNodeIds();
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
  const operatorFieldStyles = useMemo(() => {
    const styles = new Map<string, { stroke?: string; fieldDimmed: boolean }>();
    if (!nodeColoring) return styles;
    for (const op of operators) {
      if (styles.has(op.operatorId)) continue;
      if (nodeColoring.type === 'continuous') {
        const v = nodeColoring.values.get(op.operatorId);
        if (v === undefined) {
          styles.set(op.operatorId, { stroke: undefined, fieldDimmed: true });
          continue;
        }
        const t =
          nodeColoring.max > nodeColoring.min
            ? (v - nodeColoring.min) / (nodeColoring.max - nodeColoring.min)
            : 0.5;
        styles.set(op.operatorId, {
          stroke: continuousColor(t, nodePalette, isDark),
          fieldDimmed: false,
        });
      } else {
        const stroke = nodeColoring.colorMap.get(op.operatorId);
        styles.set(op.operatorId, { stroke, fieldDimmed: !stroke });
      }
    }
    return styles;
  }, [operators, nodeColoring, nodePalette, isDark]);
  const showYScroll = rowCount > MAX_VISIBLE_ROWS;
  const yAxisZoomEnd = showYScroll ? (MAX_VISIBLE_ROWS / rowCount) * 100 : 100;
  type RenderItem = NonNullable<CustomSeriesOption['renderItem']>;

  const renderItem: RenderItem = useCallback(
    (params, api) => {
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
      const { fill: fallbackFill, stroke: fallbackStroke } = getOperatorBarColors(op?.typeName);
      const fieldStyle = op ? operatorFieldStyles.get(op.operatorId) : undefined;
      const stroke = fieldStyle?.stroke ?? fallbackStroke;
      const fill = fieldStyle?.stroke ? withOpacity(stroke, 0.15) : fallbackFill;
      const hasSelection = selectedNodeIds.size > 0;
      const isSelected = op != null && selectedNodeIds.has(op.operatorId);
      const fieldDimmed = fieldStyle?.fieldDimmed ?? false;
      const opacity = fieldDimmed || (hasSelection && !isSelected) ? 0.35 : 1;

      const rect = {
        type: 'rect' as const,
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
    [operators, operatorFieldStyles, barLabelTextColor, selectedNodeIds]
  );

  const gridOptions = useMemo(
    () => ({
      ...TIMELINE_SPACING,
      top: 0,
      bottom: 0,
      left: TIMELINE_SPACING.left,
      right: TIMELINE_SPACING.right,
      width: undefined as number | undefined,
      height: undefined as number | undefined,
    }),
    []
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
    <ReactEChartsComponent
      echarts={echarts}
      theme={themeName}
      option={option}
      style={{ height }}
      onChartReady={handleChartReady}
      onEvents={handleClick}
      notMerge={false}
      lazyUpdate={false}
      replaceMerge={['series']}
    />
  );
}
