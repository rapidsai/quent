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
import EChartsReactCore from 'echarts-for-react/lib/core';

import type { EChartsInstance } from 'echarts-for-react';
import { echarts } from '../lib/echarts';
import { registerAxisPointerSync, unregisterAxisPointerSync } from '../lib/timeline.utils';
import { useChartConnect } from '../lib/useChartConnect';
import { useMinZoomSpanPct } from '../lib/useMinZoomSpanPct';
import { useTimelineWheelNavigation } from '../lib/useTimelineWheelNavigation';
import { CHART_GROUP } from '../timeline/types';
import { useTimelineEchartsTheme } from '../timeline/timelineEchartsTheme';
import { HiddenScroll } from '../ui/thin-scroll';
import { observeGanttHover, type GanttHover } from './hover';
import {
  buildGanttOption,
  type GanttDatum,
  type GanttGridSpacing,
  type GanttRenderItem,
  type GanttSeriesCursor,
} from './options';

export type { GanttDatum, GanttGridSpacing, GanttRenderItem } from './options';
type EChartsEvents = ComponentProps<typeof EChartsReactCore>['onEvents'];

export interface GanttChartProps<T extends GanttDatum> {
  data: T[];
  durationSeconds: number;
  height: number;
  maxHeight: number;
  rowHeight: number;
  isDark: boolean;
  seriesName: string;
  renderItem: GanttRenderItem;
  emptyMessage: ReactNode;
  cursor?: GanttSeriesCursor;
  onEvents?: EChartsEvents;
  gridSpacing?: GanttGridSpacing;
  contentPaddingBottom?: number;
  animateHeight?: boolean;
  renderTooltip?: (hover: GanttHover | null) => ReactNode;
  /** Called when the user clicks the chart background (not a series item). */
  onBackgroundClick?: () => void;
}

export function GanttChart<T extends GanttDatum>({
  data,
  durationSeconds,
  height,
  maxHeight,
  rowHeight,
  isDark,
  seriesName,
  renderItem,
  emptyMessage,
  cursor,
  onEvents,
  gridSpacing,
  contentPaddingBottom = 0,
  animateHeight = false,
  renderTooltip,
  onBackgroundClick,
}: GanttChartProps<T>) {
  const { themeName } = useTimelineEchartsTheme(isDark);
  const [hover, setHover] = useState<GanttHover | null>(null);
  const minZoomSpanPct = useMinZoomSpanPct(durationSeconds);
  const attachWheelNavigation = useTimelineWheelNavigation(minZoomSpanPct);
  const wrapperRef = useRef<HTMLDivElement | null>(null);
  const chartCleanupRef = useRef<(() => void) | null>(null);

  const { yAxisCategories, rowCount } = useMemo(() => {
    if (data.length === 0) return { yAxisCategories: [] as number[], rowCount: 0 };
    const maxRow = data.reduce((max, datum) => Math.max(max, datum.value[2]), 0);
    return {
      yAxisCategories: Array.from({ length: maxRow + 1 }, (_, index) => index),
      rowCount: maxRow + 1,
    };
  }, [data]);
  const chartHeight = Math.max(height, rowCount * rowHeight + contentPaddingBottom);
  const wrapperHeight = Math.min(chartHeight, maxHeight);

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
        gridSpacing,
      }),
    [
      data,
      durationSeconds,
      yAxisCategories,
      seriesName,
      renderItem,
      minZoomSpanPct,
      cursor,
      gridSpacing,
    ]
  );

  const onChartReady = useCallback(
    (instance: EChartsInstance) => {
      chartCleanupRef.current?.();
      registerAxisPointerSync(instance, 0, { receiveShowTip: false });
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
        if (!e.target) onBackgroundClick?.();
      };
      zr?.on('click', handleZrClick);

      const cleanup = () => {
        unregisterAxisPointerSync(instance);
        detachWheelNavigation();
        detachHover?.();
        zr?.off('click', handleZrClick);
        if (chartCleanupRef.current === cleanup) chartCleanupRef.current = null;
      };
      chartCleanupRef.current = cleanup;
    },
    [attachWheelNavigation, renderTooltip, onBackgroundClick]
  );

  const { handleChartReady, instanceRef } = useChartConnect({
    durationSeconds,
    chartGroup: CHART_GROUP,
    onReady: onChartReady,
  });

  useEffect(() => {
    return () => {
      chartCleanupRef.current?.();
      instanceRef.current = null;
    };
  }, [instanceRef]);

  return (
    <>
      <HiddenScroll
        ref={wrapperRef}
        className={
          animateHeight
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
          <div className="pointer-events-none absolute inset-0 flex items-center justify-center text-sm text-muted-foreground">
            {emptyMessage}
          </div>
        )}
      </HiddenScroll>
      {data.length > 0 && renderTooltip?.(hover)}
    </>
  );
}
