// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useLayoutEffect, useMemo, useRef, useState } from 'react';
import type { NvtxLane } from '@quent/utils';
import { withOpacity } from '@quent/utils';
import { useDebouncedZoomRange } from '@quent/hooks';
import {
  MARK_AREA_BORDER_OPACITY,
  MARK_AREA_FILL_OPACITY,
  useTimelineEchartsTheme,
} from '../timeline/timelineEchartsTheme';
import { TIMELINE_SPACING } from '../timeline/types';
import { EntityTooltipContent } from '../timeline/TimelineTooltip';
import { GanttChart, type GanttRenderItem } from '../gantt-chart/GanttChart';
import type { GanttHover } from '../gantt-chart/hover';
import { GANTT_RESIZE_CONTROL_HEIGHT, layoutGanttBar } from '../gantt-chart/utils';
import { PointerTooltipPortal } from '../ui/pointer-tooltip-portal';
import {
  mergeNvtxGanttData,
  NVTX_MIN_BAR_WIDTH_PX,
  nvtxItemsAtTimestamp,
  nvtxLanesToGanttData,
  nvtxMergedBarCountLabel,
  nvtxTooltipModel,
  rgbHex,
} from './utils';

const BAR_FONT_SIZE = 10;
const BAR_HEIGHT = 14;
const BAR_GAP = 2;
const ROW_HEIGHT = BAR_HEIGHT + BAR_GAP;
const NVTX_COLLAPSED_ROWS = 3;
/** Collapsed height that keeps three nested depths visible, including the expand control. */
export const NVTX_GANTT_HEIGHT =
  NVTX_COLLAPSED_ROWS * ROW_HEIGHT +
  TIMELINE_SPACING.top +
  TIMELINE_SPACING.bottom +
  GANTT_RESIZE_CONTROL_HEIGHT;
const SERIES_NAME = 'nvtx-range';

export interface NvtxGanttProps {
  lanes: NvtxLane[];
  durationSeconds: number;
  height?: number;
  isDark: boolean;
}

export function NvtxGantt({
  lanes,
  durationSeconds,
  height = NVTX_GANTT_HEIGHT,
  isDark,
}: NvtxGanttProps) {
  const { textColor } = useTimelineEchartsTheme(isDark);
  const zoomRange = useDebouncedZoomRange();
  const containerRef = useRef<HTMLDivElement>(null);
  const [plotWidthPx, setPlotWidthPx] = useState(0);

  useLayoutEffect(() => {
    const element = containerRef.current;
    if (!element || typeof ResizeObserver === 'undefined') {
      return;
    }
    const update = () => setPlotWidthPx(Math.max(0, element.clientWidth - TIMELINE_SPACING.right));
    update();
    const observer = new ResizeObserver(update);
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  const visibleStartMs = (zoomRange.end > zoomRange.start ? zoomRange.start : 0) * 1_000;
  const visibleEndMs = (zoomRange.end > zoomRange.start ? zoomRange.end : durationSeconds) * 1_000;
  const minimumHitWidthMs =
    plotWidthPx > 0 ? ((visibleEndMs - visibleStartMs) * NVTX_MIN_BAR_WIDTH_PX) / plotWidthPx : 0;

  const customSeriesData = useMemo(() => {
    const data = nvtxLanesToGanttData(lanes);
    return mergeNvtxGanttData(data, { visibleStartMs, visibleEndMs, plotWidthPx });
  }, [lanes, visibleStartMs, visibleEndMs, plotWidthPx]);

  const renderTooltip = useCallback(
    (hover: GanttHover | null) => {
      const tooltip = hover
        ? nvtxTooltipModel(
            nvtxItemsAtTimestamp(customSeriesData, hover.timestampMs, minimumHitWidthMs)
          )
        : null;
      return (
        <PointerTooltipPortal hover={tooltip && tooltip.marks.length > 0 ? hover : null}>
          {hover && tooltip && (
            <EntityTooltipContent
              timestamp={hover.timestampMs}
              windowMs={(zoomRange.end - zoomRange.start) * 1_000}
              activeMarks={tooltip.marks}
              itemLimit={tooltip.itemLimit}
              itemNoun={tooltip.itemNoun}
              summary={tooltip.summary}
              className="max-w-[20rem]"
            />
          )}
        </PointerTooltipPortal>
      );
    },
    [customSeriesData, minimumHitWidthMs, zoomRange.end, zoomRange.start]
  );

  const renderItem: GanttRenderItem = useCallback(
    (params, api) => {
      const datum = customSeriesData[params.dataIndex];
      if (!datum) {
        return null;
      }
      const layout = layoutGanttBar(params, api, {
        barHeight: BAR_HEIGHT,
        minWidth: NVTX_MIN_BAR_WIDTH_PX,
        allowInstant: true,
      });
      if (!layout) {
        return null;
      }
      const { clippedShape } = layout;
      const color = rgbHex(datum.range?.color ?? datum.mark?.color ?? '#2563eb');
      const merged = (datum.mergedCount ?? 1) > 1;
      const label = merged ? '' : (datum.range?.message ?? datum.mark?.message ?? '');
      const rect = {
        type: 'rect' as const,
        shape: { ...clippedShape, r: datum.mark ? 0 : 2 },
        style: {
          fill: withOpacity(color, MARK_AREA_FILL_OPACITY),
          stroke: withOpacity(color, MARK_AREA_BORDER_OPACITY),
          lineWidth: 1,
          ...(merged ? { lineDash: [2, 1] } : {}),
        },
      };
      const text =
        label && clippedShape.width > 10
          ? {
              type: 'text' as const,
              style: {
                text: label,
                x: clippedShape.x + 6,
                y: clippedShape.y + clippedShape.height / 2,
                textVerticalAlign: 'middle' as const,
                fontSize: BAR_FONT_SIZE,
                fill: textColor,
                overflow: 'truncate' as const,
                width: Math.max(0, clippedShape.width - 12),
              },
            }
          : null;
      const marker = merged
        ? nvtxMergedBarCountLabel(
            clippedShape,
            textColor,
            datum.mergedCount ?? 1,
            datum.mark ? 'mark' : 'range'
          )
        : [];
      return {
        type: 'group' as const,
        children: text ? [rect, text, ...marker] : [rect, ...marker],
      };
    },
    [customSeriesData, textColor]
  );

  return (
    <div ref={containerRef} className="h-full w-full">
      <GanttChart
        data={customSeriesData}
        durationSeconds={durationSeconds}
        height={height}
        maxHeight={height}
        rowHeight={ROW_HEIGHT}
        isDark={isDark}
        seriesName={SERIES_NAME}
        renderItem={renderItem}
        expandable
        expandLabel="Expand NVTX chart"
        collapseLabel="Collapse NVTX chart"
        emptyMessage="No NVTX ranges"
        showPlayhead
        renderTooltip={renderTooltip}
      />
    </div>
  );
}
