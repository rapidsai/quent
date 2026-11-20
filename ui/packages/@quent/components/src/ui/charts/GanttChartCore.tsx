// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ComponentProps,
  type ReactNode,
} from 'react';
import { ChevronDown, ChevronUp } from 'lucide-react';

import type { EChartsInstance } from 'echarts-for-react';
import { echarts } from './echarts';
import { EChartsReactCore } from './echartsReactCore';
import { useMinZoomSpanPct } from './useMinZoomSpanPct';
import { useChartWheelNavigation } from './useChartWheelNavigation';
import { ChartPointerArea } from './ChartPointerArea';
import { TIMELINE_SPACING } from './timelineTypes';
import { useTimelineEchartsTheme } from './timelineTheme';
import { Button } from '../button';
import { HiddenScroll } from '../thin-scroll';
import { observeGanttHover, type GanttHover } from './ganttHover';
import {
  buildGanttOption,
  type GanttDatum,
  type GanttGridSpacing,
  type GanttRenderItem,
  type GanttSeriesCursor,
} from './ganttOptions';
import { ganttExpansionLayout } from './ganttLayout';

export type { GanttDatum, GanttGridSpacing, GanttRenderItem } from './ganttOptions';
type EChartsEvents = ComponentProps<typeof EChartsReactCore>['onEvents'];

export interface GanttChartCoreProps<T extends GanttDatum> {
  data: T[];
  durationSeconds: number;
  height: number;
  maxHeight: number;
  rowHeight: number;
  isDark: boolean;
  seriesName: string;
  renderItem: GanttRenderItem;
  emptyState: ReactNode;
  cursor?: GanttSeriesCursor;
  onEvents?: EChartsEvents;
  gridSpacing?: GanttGridSpacing;
  contentPaddingBottom?: number;
  animateHeight?: boolean;
  /** Grow the row to fit stacked lanes instead of scrolling inside a fixed max height. */
  expandable?: boolean;
  expandLabel?: string;
  collapseLabel?: string;
  renderTooltip?: (hover: GanttHover | null) => ReactNode;
  /** Called when the user clicks the chart background (not a series item). */
  onBackgroundClick?: () => void;
  minZoomWindowSeconds: number;
  pointerRatio: number | null;
  onPointerRatioChange: (ratio: number | null) => void;
  onChartReady?: (instance: EChartsInstance) => void;
  renderOverlay?: (instance: EChartsInstance | null) => ReactNode;
}

export function GanttChartCore<T extends GanttDatum>({
  data,
  durationSeconds,
  height,
  maxHeight,
  rowHeight,
  isDark,
  seriesName,
  renderItem,
  emptyState,
  cursor,
  onEvents,
  gridSpacing,
  contentPaddingBottom = 0,
  animateHeight = false,
  expandable = false,
  expandLabel = 'Expand chart',
  collapseLabel = 'Collapse chart',
  renderTooltip,
  onBackgroundClick,
  minZoomWindowSeconds,
  pointerRatio,
  onPointerRatioChange,
  onChartReady,
  renderOverlay,
}: GanttChartCoreProps<T>) {
  const { themeName } = useTimelineEchartsTheme(isDark);
  const [hover, setHover] = useState<GanttHover | null>(null);
  const [isExpanded, setIsExpanded] = useState(false);
  const [chartInstance, setChartInstance] = useState<EChartsInstance | null>(null);
  const minZoomSpanPct = useMinZoomSpanPct(durationSeconds, minZoomWindowSeconds);
  const attachWheelNavigation = useChartWheelNavigation(minZoomSpanPct, TIMELINE_SPACING);
  const wrapperRef = useRef<HTMLDivElement | null>(null);
  const chartCleanupRef = useRef<(() => void) | null>(null);

  const { yAxisCategories, rowCount } = useMemo(() => {
    if (data.length === 0) {
      return { yAxisCategories: [] as number[], rowCount: 0 };
    }
    const maxRow = data.reduce((max, datum) => Math.max(max, datum.value[2]), 0);
    return {
      yAxisCategories: Array.from({ length: maxRow + 1 }, (_, index) => index),
      rowCount: maxRow + 1,
    };
  }, [data]);
  const expansion = expandable
    ? ganttExpansionLayout({
        rowCount,
        rowHeight,
        collapsedHeight: height,
        isExpanded,
      })
    : null;
  const resolvedMaxHeight = expansion?.maxHeight ?? maxHeight;
  const resolvedPadding = expansion?.contentPaddingBottom ?? contentPaddingBottom;
  const resolvedGridSpacing = expansion?.gridSpacing ?? gridSpacing;
  const pointerLeft =
    typeof resolvedGridSpacing?.left === 'number'
      ? resolvedGridSpacing.left
      : TIMELINE_SPACING.left;
  const pointerRight =
    typeof resolvedGridSpacing?.right === 'number'
      ? resolvedGridSpacing.right
      : TIMELINE_SPACING.right;
  const chartHeight =
    expansion?.contentHeight ?? Math.max(height, rowCount * rowHeight + resolvedPadding);
  const wrapperHeight = Math.min(chartHeight, resolvedMaxHeight);
  const shouldAnimateHeight = expandable || animateHeight;

  const option = useMemo(
    () =>
      buildGanttOption({
        data,
        durationSeconds,
        yAxisCategories,
        seriesName,
        renderItem,
        minZoomSpanPct,
        cursor,
        gridSpacing: resolvedGridSpacing,
      }),
    [
      data,
      durationSeconds,
      yAxisCategories,
      seriesName,
      renderItem,
      minZoomSpanPct,
      cursor,
      resolvedGridSpacing,
    ]
  );

  const handleChartReady = useCallback(
    (instance: EChartsInstance) => {
      onChartReady?.(instance);
      setChartInstance(instance);
      chartCleanupRef.current?.();
      const detachWheelNavigation = attachWheelNavigation(
        instance,
        wrapperRef.current ?? undefined
      );
      const detachHover = renderTooltip ? observeGanttHover(instance, setHover) : undefined;

      // zrender fires click for ALL clicks; target is null when background is clicked
      type ZrEvent = { target: unknown };
      const zr = (
        instance as unknown as {
          getZr: () => {
            on: (e: string, h: (ev: ZrEvent) => void) => void;
            off: (e: string, h: (ev: ZrEvent) => void) => void;
          };
        }
      ).getZr?.();
      const handleZrClick = (e: ZrEvent) => {
        if (!e.target) {
          onBackgroundClick?.();
        }
      };
      zr?.on('click', handleZrClick);

      const cleanup = () => {
        detachWheelNavigation();
        detachHover?.();
        zr?.off('click', handleZrClick);
        if (chartCleanupRef.current === cleanup) {
          chartCleanupRef.current = null;
        }
      };
      chartCleanupRef.current = cleanup;
    },
    [attachWheelNavigation, onBackgroundClick, onChartReady, renderTooltip]
  );

  useEffect(() => {
    return () => {
      chartCleanupRef.current?.();
    };
  }, []);

  return (
    <ChartPointerArea
      left={pointerLeft}
      right={pointerRight}
      pointerRatio={pointerRatio}
      onPointerRatioChange={onPointerRatioChange}
    >
      <HiddenScroll
        ref={wrapperRef}
        className={
          shouldAnimateHeight
            ? 'relative transition-[height] duration-150 ease-out motion-reduce:transition-none'
            : 'relative'
        }
        style={{ height: wrapperHeight }}
      >
        <EChartsReactCore
          echarts={echarts}
          theme={themeName}
          option={option}
          style={{ height: chartHeight }}
          onChartReady={handleChartReady}
          onEvents={onEvents}
          notMerge={false}
          lazyUpdate={false}
          replaceMerge={['series']}
          autoResize={false}
        />
        {data.length === 0 && (
          <div className="pointer-events-none absolute inset-0">{emptyState}</div>
        )}
      </HiddenScroll>
      {renderOverlay?.(chartInstance)}
      {expansion?.canResize && (
        <Button
          type="button"
          variant="ghost"
          size="xs"
          className="absolute bottom-0 left-0 z-10 h-3 rounded-none border-t border-border/50 bg-background/90 p-0 text-muted-foreground backdrop-blur-sm focus-visible:bg-accent focus-visible:ring-0 focus-visible:ring-offset-0 [&_svg]:size-3"
          style={{ right: expansion.gridSpacing.right }}
          aria-label={isExpanded ? collapseLabel : expandLabel}
          aria-expanded={isExpanded}
          onClick={event => {
            event.stopPropagation();
            setIsExpanded(current => !current);
          }}
        >
          {isExpanded ? <ChevronUp /> : <ChevronDown />}
        </Button>
      )}
      {data.length > 0 && renderTooltip?.(hover)}
    </ChartPointerArea>
  );
}
