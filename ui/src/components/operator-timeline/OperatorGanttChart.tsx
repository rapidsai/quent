import { useCallback, useEffect, useMemo, useRef } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import ReactECharts from 'echarts-for-react/lib/core';
import { echarts } from '@/lib/echarts';

/** Clip a rect to bounds (same behavior as ECharts custom-gantt-flight example). */
function clipRectByRect(
  target: { x: number; y: number; width: number; height: number },
  bounds: { x: number; y: number; width: number; height: number }
): { x: number; y: number; width: number; height: number } | undefined {
  const x = Math.max(target.x, bounds.x);
  const x2 = Math.min(target.x + target.width, bounds.x + bounds.width);
  const y = Math.max(target.y, bounds.y);
  const y2 = Math.min(target.y + target.height, bounds.y + bounds.height);
  if (x2 >= x && y2 >= y) {
    return { x, y, width: x2 - x, height: y2 - y };
  }
  return undefined;
}
import type { EChartsOption } from '@/lib/echarts';
import type { EChartsInstance } from 'echarts-for-react';
import { useAtomValue } from 'jotai';
import {
  connectChart,
  nanosToMs,
  registerAxisPointerSync,
  unregisterAxisPointerSync,
} from '@/lib/timeline.utils';
import { CHART_GROUP } from '@/components/timeline/Timeline';
import { useTimelineChartColors } from '@/components/timeline/useTimelineChartColors';
import { zoomRangeAtom } from '@/atoms/timeline';
import { getColorForKey } from '@/services/colors';
import type { OperatorActiveSpanEntry } from './types';
import { TIMELINE_SPACING } from '@/components/timeline/types';
import { formatDurationForWindow } from '@/services/formatters';

const DEFAULT_HEIGHT = 75;

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
  const { gridBorderColor, gridBackgroundColor, timelineMarkupColor } = useTimelineChartColors();
  const zoomRange = useAtomValue(zoomRangeAtom);
  const windowMsRef = useRef(0);
  windowMsRef.current = (zoomRange.end - zoomRange.start) * 1000;
  const cursorTimestampMsRef = useRef<number>(0);

  const startTimeMs = useMemo(() => nanosToMs(startTime), [startTime]);
  const xAxisMax = useMemo(
    () => startTimeMs + durationSeconds * 1_000,
    [startTimeMs, durationSeconds]
  );

  const yAxisCategories = useMemo(() => operators.map((_, i) => i), [operators]);

  const customSeriesData = useMemo(
    () =>
      operators.map(op => ({
        value: [op.startMs, op.endMs, op.rowIndex] as [number, number, number],
        name: op.label,
      })),
    [operators]
  );

  const barColor = useMemo(() => getColorForKey('operator'), []);

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
      const sizeResult = api.size?.([0, 1]);
      const bandHeight = Array.isArray(sizeResult)
        ? sizeResult[1]
        : typeof sizeResult === 'number'
          ? sizeResult
          : 20;
      const barHeight = Math.max(4, bandHeight * 0.7);
      const y = startPoint[1] - barHeight / 2;
      const width = Math.max(1, endPoint[0] - startPoint[0]);

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
      const clippedShape = clipBound
        ? clipRectByRect(rectShape, clipBound)
        : rectShape;
      if (!clippedShape) return null;

      const op = operators[params.dataIndexInside];
      const barLabel =
        op?.typeName && op.typeName !== op.label
          ? `${op.typeName}: ${op.label}`
          : (op?.label ?? '');

      const rect = {
        type: 'rect' as const,
        shape: clippedShape,
        style: {
          fill: barColor,
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
          fontSize: 10,
          fill: '#fff',
          overflow: 'truncate' as const,
          width: Math.max(0, clippedShape.width - 12),
        },
      };

      return {
        type: 'group' as const,
        children: [rect, text],
      };
    },
    [barColor, operators]
  );

  const gridOptions = useMemo(
    () => ({
      ...TIMELINE_SPACING,
      top: TIMELINE_SPACING.top + 10,
      bottom: TIMELINE_SPACING.bottom + 10,
      left: TIMELINE_SPACING.left,
      right: TIMELINE_SPACING.right,
      width: undefined as number | undefined,
      height: undefined as number | undefined,
      backgroundColor: gridBackgroundColor,
      borderWidth: 1,
      borderColor: gridBorderColor,
      show: true,
    }),
    [gridBackgroundColor, gridBorderColor]
  );

  const option: EChartsOption = useMemo(
    () => ({
      animation: false,
      tooltip: {
        trigger: 'item',
        transitionDuration: 0,
        backgroundColor: 'transparent',
        borderWidth: 0,
        padding: 0,
        confine: true,
        appendToBody: true,
        formatter: (params: unknown) => {
          const p = params as { name: string; dataIndex: number };
          const timestampMs =
            cursorTimestampMsRef.current > 0 ? cursorTimestampMsRef.current : null;
          const offsetMs = timestampMs != null ? timestampMs - startTimeMs : 0;
          const timeStr =
            timestampMs != null ? formatDurationForWindow(offsetMs, windowMsRef.current) : '';
          const op = operators[p.dataIndex];
          const operatorLabel =
            op && op.typeName && op.typeName !== op.label ? `${op.typeName}: ${op.label}` : p.name;
          return renderToStaticMarkup(
            <div className="px-2 py-1.5 bg-popover rounded text-[11px] text-foreground leading-tight shadow-md z-50 min-w-[120px]">
              {timeStr ? (
                <div className="font-semibold text-muted-foreground mb-1">{timeStr}</div>
              ) : null}
              <div>{operatorLabel}</div>
            </div>
          );
        },
      },
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
          data: customSeriesData,
          renderItem: renderItem as never,
          coordinateSystem: 'cartesian2d',
        },
      ],
      dataZoom: [
        { type: 'slider', show: false, realtime: true, filterMode: 'none' },
        {
          type: 'inside',
          zoomLock: true,
          zoomOnMouseWheel: false,
          throttle: 30,
          filterMode: 'none',
        },
        {
          type: 'inside',
          zoomOnMouseWheel: 'shift',
          moveOnMouseMove: false,
          moveOnMouseWheel: false,
          throttle: 30,
          filterMode: 'none',
        },
      ],
    }),
    [
      gridOptions,
      startTimeMs,
      xAxisMax,
      yAxisCategories,
      customSeriesData,
      renderItem,
      gridBorderColor,
      timelineMarkupColor,
      operators,
    ]
  );

  const instanceRef = useRef<EChartsInstance | null>(null);

  const handleChartReady = useCallback((instance: EChartsInstance) => {
    instanceRef.current = instance;
    connectChart(instance, CHART_GROUP, false);
    registerAxisPointerSync(instance, 0, { receiveShowTip: false });
    const dom = instance.getDom();
    const zr = instance.getZr();
    zr.on('mousemove', (e: { offsetX: number }) => {
      try {
        const value = instance.convertFromPixel({ xAxisIndex: 0 }, e.offsetX) as number;
        if (value != null && isFinite(value)) {
          cursorTimestampMsRef.current = value;
        }
      } catch {
        // ignore when out of range
      }
    });
    zr.on('globalout', () => {
      cursorTimestampMsRef.current = 0;
    });
    dom.addEventListener('pointerdown', () => {
      instance.dispatchAction({ type: 'hideTip' });
    });
    dom.addEventListener(
      'wheel',
      (e: WheelEvent) => {
        if (!e.shiftKey) e.stopPropagation();
      },
      { capture: true, passive: true }
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
    <ReactECharts
      echarts={echarts}
      option={option}
      style={{ height }}
      onChartReady={handleChartReady}
      notMerge
    />
  );
}
