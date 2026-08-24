// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo } from 'react';
import { useTimelineHover, useZoomRange } from '@quent/hooks';
import { TooltipContent } from './TimelineTooltip';
import type { TimelineMark, TimelineSeries } from './types';
import { PositionedTooltip } from '../ui/positioned-tooltip';

/**
 * Pointer-driven tooltip rendered as a single body-level portal.
 *
 * Each Timeline mounts one of these guarded by `sourceId === ownerId`, so at
 * most one portal ever renders DOM at a time.
 */
export function TimelineTooltipPortal({
  ownerId,
  series,
  timestamps,
  marks,
}: {
  /** Stable id of the Timeline that owns this portal. */
  ownerId: string;
  series: TimelineSeries;
  timestamps: number[];
  marks?: TimelineMark[];
}) {
  const hover = useTimelineHover();
  const zoomRange = useZoomRange();
  const isOwned = hover?.sourceId === ownerId;

  if (!isOwned || !hover) return null;
  if (timestamps.length === 0) return null;

  // Defensive clamp: a stale `dataIndex` from a previous render could exceed
  // the current array length
  const dataIndex = Math.max(0, Math.min(timestamps.length - 1, hover.dataIndex));

  return (
    <TimelineTooltipAtPosition
      clientX={hover.clientX}
      clientY={hover.clientY}
      dataIndex={dataIndex}
      series={series}
      timestamps={timestamps}
      marks={marks}
      windowMs={(zoomRange.end - zoomRange.start) * 1000}
    />
  );
}

function TimelineTooltipAtPosition({
  clientX,
  clientY,
  dataIndex,
  series,
  timestamps,
  marks,
  windowMs,
}: {
  clientX: number;
  clientY: number;
  dataIndex: number;
  series: TimelineSeries;
  timestamps: number[];
  marks?: TimelineMark[];
  windowMs: number;
}) {
  const { snappedTimestamp, tooltipSeries, activeMarks } = useMemo(() => {
    const snapped = timestamps[dataIndex] ?? 0;
    const tooltipSeriesValues = Object.entries(series).map(([name, entry]) => ({
      color: entry.color,
      name,
      value: entry.values[dataIndex] ?? 0,
      isOverlay: entry.isOverlay ?? false,
      isDimmed: entry.isDimmed ?? false,
    }));
    const activeMarksAtTs = marks
      ?.filter(m => snapped >= m.xStart && snapped <= m.xEnd)
      .map(m => ({
        label: m.label,
        stateName: m.stateName,
        color: m.color,
        attributes: m.attributes,
        derivedAttributes: m.derivedAttributes,
        durationMs: m.xEnd - m.xStart,
      }));
    return {
      snappedTimestamp: snapped,
      tooltipSeries: tooltipSeriesValues,
      activeMarks: activeMarksAtTs && activeMarksAtTs.length > 0 ? activeMarksAtTs : undefined,
    };
  }, [series, timestamps, marks, dataIndex]);

  const fmt = useMemo(() => Object.values(series)[0]?.formatter, [series]);

  return (
    <PositionedTooltip clientX={clientX} clientY={clientY}>
      <TooltipContent
        timestamp={snappedTimestamp}
        series={tooltipSeries}
        fmt={fmt}
        windowMs={windowMs}
        activeMarks={activeMarks}
      />
    </PositionedTooltip>
  );
}
