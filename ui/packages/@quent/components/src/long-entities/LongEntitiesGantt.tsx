// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useMemo } from 'react';

import {
  MARK_AREA_BORDER_OPACITY,
  MARK_AREA_FILL_OPACITY,
  useTimelineEchartsTheme,
} from '../timeline/timelineEchartsTheme';
import { useZoomRange } from '@quent/hooks';
import { formatDuration, withOpacity } from '@quent/utils';
import type { LongEntityEntry } from './types';
import { GanttChart, type GanttRenderItem } from '../gantt-chart/GanttChart';
import type { GanttHover } from '../gantt-chart/hover';
import { layoutGanttBar } from '../gantt-chart/utils';
import { getLongEntitySegmentsAtTimestamp } from './utils';
import { PointerTooltipPortal } from '../ui/pointer-tooltip-portal';
import { EntityTooltipContent, type ActiveMark } from '../timeline/TimelineTooltip';

export const LONG_ENTITIES_TIMELINE_HEIGHT = 75;
const LABEL_FONT_SIZE = 9;
const BAR_HEIGHT = LABEL_FONT_SIZE + 4;
/** Vertical gap between stacked rows. */
const ROW_GAP = 1;
const ROW_HEIGHT = BAR_HEIGHT + ROW_GAP;
/** Radius applied only to the outer corners of each entity's segment run. */
const CORNER_RADIUS = 2;
const SERIES_NAME = 'long-entity-segment';

/** Flat segment datum: one ECharts custom-series item per state span. */
type SegmentDatum = {
  value: [number, number, number];
  entryIndex: number;
  segmentIndex: number;
};

export interface LongEntitiesGanttProps {
  entries: LongEntityEntry[];
  durationSeconds: number;
  minUsageSeconds: number;
  height?: number;
  /** Whether dark mode is active. Passed explicitly to decouple from ThemeContext. */
  isDark: boolean;
  onEntityClick?: (entry: LongEntityEntry) => void;
  /** When set, dims all entity bars except the one with this entity ID. */
  selectedEntityId?: string;
  /** Called when the user clicks the chart background (not an entity bar). */
  onBackgroundClick?: () => void;
}

export function LongEntitiesGantt({
  entries,
  durationSeconds,
  minUsageSeconds,
  height = LONG_ENTITIES_TIMELINE_HEIGHT,
  isDark,
  onEntityClick,
  selectedEntityId,
  onBackgroundClick,
}: LongEntitiesGanttProps) {
  const { textColor } = useTimelineEchartsTheme(isDark);
  const zoomRange = useZoomRange();
  // One custom-series datum per segment, tagged with its parent entry/segment.
  const customSeriesData = useMemo<SegmentDatum[]>(() => {
    const data: SegmentDatum[] = [];
    entries.forEach((entry, entryIndex) => {
      entry.segments.forEach((seg, segmentIndex) => {
        data.push({
          value: [seg.startMs, seg.endMs, entry.rowIndex],
          entryIndex,
          segmentIndex,
        });
      });
    });
    return data;
  }, [entries]);
  const renderTooltip = useCallback(
    (hover: GanttHover | null) => {
      const activeMarks: ActiveMark[] = hover
        ? getLongEntitySegmentsAtTimestamp(entries, hover.timestampMs).map(
            ({ entry, segment }) => ({
              color: segment.color,
              label: entry.label,
              stateName: segment.stateName,
              durationMs: segment.endMs - segment.startMs,
              attributes: segment.attributes,
              derivedAttributes: segment.derivedAttributes,
            })
          )
        : [];
      return (
        <PointerTooltipPortal hover={activeMarks.length > 0 ? hover : null}>
          {hover && (
            <EntityTooltipContent
              timestamp={hover.timestampMs}
              windowMs={(zoomRange.end - zoomRange.start) * 1_000}
              activeMarks={activeMarks}
            />
          )}
        </PointerTooltipPortal>
      );
    },
    [entries, zoomRange.end, zoomRange.start]
  );

  const renderItem: GanttRenderItem = useCallback(
    (params, api) => {
      const datum = customSeriesData[params.dataIndex];
      const entry = datum ? entries[datum.entryIndex] : undefined;
      const segment = entry?.segments[datum!.segmentIndex];
      if (!entry || !segment) {
        return null;
      }

      const layout = layoutGanttBar(params, api, { barHeight: BAR_HEIGHT });
      if (!layout) {
        return null;
      }
      const { clippedShape } = layout;

      const hasSelection = selectedEntityId != null;
      const isSelected = hasSelection && entry.entityId === selectedEntityId;
      const opacity = hasSelection && !isSelected ? 0.3 : 1;

      const color = segment.color;
      const isFirst = datum!.segmentIndex === 0;
      const isLast = datum!.segmentIndex === entry.segments.length - 1;
      // [topLeft, topRight, bottomRight, bottomLeft] — round only the run's outer corners
      // so touching segments tile with square inner seams.
      const r: [number, number, number, number] = [
        isFirst ? CORNER_RADIUS : 0,
        isLast ? CORNER_RADIUS : 0,
        isLast ? CORNER_RADIUS : 0,
        isFirst ? CORNER_RADIUS : 0,
      ];
      const rect = {
        type: 'rect' as const,
        shape: { ...clippedShape, r },
        // Mirror timeline marks: faint fill, stronger border, same state color.
        style: {
          fill: withOpacity(color, MARK_AREA_FILL_OPACITY),
          stroke: withOpacity(color, MARK_AREA_BORDER_OPACITY),
          lineWidth: 1,
          opacity,
        },
      };

      const labelChildren =
        clippedShape.width > 10
          ? [
              {
                type: 'text' as const,
                style: {
                  text: `${entry.label} (${segment.stateName})`,
                  x: clippedShape.x + clippedShape.width / 2,
                  y: clippedShape.y + clippedShape.height / 2,
                  textAlign: 'center' as const,
                  textVerticalAlign: 'middle' as const,
                  fontSize: LABEL_FONT_SIZE,
                  fill: textColor,
                  overflow: 'truncate' as const,
                  width: Math.max(0, clippedShape.width - 6),
                  opacity,
                },
              },
            ]
          : [];

      return { type: 'group' as const, children: [rect, ...labelChildren] };
    },
    [entries, customSeriesData, textColor, selectedEntityId]
  );

  const onEvents = useMemo(() => {
    if (!onEntityClick) {
      return undefined;
    }
    return {
      click: (params: { dataIndex: number; seriesName?: string }) => {
        if (params.seriesName !== SERIES_NAME) {
          return;
        }
        const datum = customSeriesData[params.dataIndex];
        if (!datum) {
          return;
        }
        const entry = entries[datum.entryIndex];
        if (entry) {
          onEntityClick(entry);
        }
      },
    };
  }, [onEntityClick, customSeriesData, entries]);

  return (
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
      expandLabel="Expand entities chart"
      collapseLabel="Collapse entities chart"
      emptyMessage={
        <div className="flex flex-col items-center gap-0.5 text-center text-muted-foreground opacity-50">
          <div className="font-medium">No Matching Entities</div>
          <div className="text-xs">
            Showing entities longer than {formatDuration(minUsageSeconds * 1_000, 1)}. Zoom to see
            more.
          </div>
        </div>
      }
      renderTooltip={renderTooltip}
      cursor={onEntityClick ? 'pointer' : undefined}
      onEvents={onEvents}
      onBackgroundClick={onBackgroundClick}
    />
  );
}
