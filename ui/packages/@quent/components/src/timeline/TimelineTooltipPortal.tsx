// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useLayoutEffect, useMemo, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { useTimelineHover, useZoomRange } from '@quent/hooks';
import { TooltipContent } from './TimelineTooltip';
import type { TimelineMark, TimelineSeries } from './types';

const POINTER_OFFSET = 12;
const VIEWPORT_MARGIN = 4;

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
  startTime,
}: {
  /** Stable id of the Timeline that owns this portal. */
  ownerId: string;
  series: TimelineSeries;
  timestamps: number[];
  marks?: TimelineMark[];
  startTime: bigint;
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
    <PositionedTooltip
      clientX={hover.clientX}
      clientY={hover.clientY}
      dataIndex={dataIndex}
      series={series}
      timestamps={timestamps}
      marks={marks}
      startTime={startTime}
      windowMs={(zoomRange.end - zoomRange.start) * 1000}
    />
  );
}

function PositionedTooltip({
  clientX,
  clientY,
  dataIndex,
  series,
  timestamps,
  marks,
  startTime,
  windowMs,
}: {
  clientX: number;
  clientY: number;
  dataIndex: number;
  series: TimelineSeries;
  timestamps: number[];
  marks?: TimelineMark[];
  startTime: bigint;
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
        pipeline: m.pipeline,
        durationMs: m.xEnd - m.xStart,
      }));
    return {
      snappedTimestamp: snapped,
      tooltipSeries: tooltipSeriesValues,
      activeMarks: activeMarksAtTs && activeMarksAtTs.length > 0 ? activeMarksAtTs : undefined,
    };
  }, [series, timestamps, marks, dataIndex]);

  const fmt = useMemo(() => Object.values(series)[0]?.formatter, [series]);

  const hostRef = useRef<HTMLDivElement | null>(null);
  // Defer-clamp to viewport: render once at the raw position, measure, then
  // adjust if the box would overflow. Two-phase keeps us simple — confine: true
  // was free with ECharts; here it's ~10 lines.
  const [position, setPosition] = useState({
    left: clientX + POINTER_OFFSET,
    top: clientY + POINTER_OFFSET,
  });
  useLayoutEffect(() => {
    const el = hostRef.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    let left = clientX + POINTER_OFFSET;
    let top = clientY + POINTER_OFFSET;
    if (left + rect.width + VIEWPORT_MARGIN > vw) {
      left = Math.max(VIEWPORT_MARGIN, clientX - rect.width - POINTER_OFFSET);
    }
    if (top + rect.height + VIEWPORT_MARGIN > vh) {
      top = Math.max(VIEWPORT_MARGIN, clientY - rect.height - POINTER_OFFSET);
    }
    setPosition({ left, top });
  }, [clientX, clientY, snappedTimestamp]);

  return createPortal(
    <div
      ref={hostRef}
      style={{
        position: 'fixed',
        left: position.left,
        top: position.top,
        pointerEvents: 'none',
        zIndex: 1000,
      }}
    >
      <TooltipContent
        timestamp={snappedTimestamp}
        series={tooltipSeries}
        startTime={startTime}
        fmt={fmt}
        windowMs={windowMs}
        activeMarks={activeMarks}
      />
    </div>,
    document.body
  );
}
