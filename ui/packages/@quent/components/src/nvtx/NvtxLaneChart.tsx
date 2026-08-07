// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { useZoomRange } from '@quent/hooks';
import type { NvtxLane, NvtxMarkItem, NvtxRangeItem } from '@quent/utils';
import { GanttChart, type GanttRenderItem } from '../gantt-chart/GanttChart';
import { clipRectByRect } from '../gantt-chart/utils';
import { formatNvtxDuration, nvtxRelativeSecondsToMs } from './NvtxLaneChart.utils';

type NvtxDatum = {
  value: [number, number, number];
  type: 'range' | 'mark';
  laneIndex: number;
  itemIndex: number;
  depth: number;
};

type ActiveItem =
  | { type: 'range'; item: NvtxRangeItem; x: number; y: number }
  | { type: 'mark'; item: NvtxMarkItem; x: number; y: number };

function NvtxItemTooltip({ active }: { active: ActiveItem }) {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const [position, setPosition] = useState({ left: active.x + 12, top: active.y + 12 });

  useLayoutEffect(() => {
    const host = hostRef.current;
    if (!host) return;
    const rect = host.getBoundingClientRect();
    const margin = 4;
    const offset = 12;
    let left = active.x + offset;
    let top = active.y + offset;
    if (left + rect.width + margin > window.innerWidth) {
      left = Math.max(margin, active.x - rect.width - offset);
    }
    if (top + rect.height + margin > window.innerHeight) {
      top = Math.max(margin, active.y - rect.height - offset);
    }
    setPosition({ left, top });
  }, [active]);

  const common = active.item;
  return createPortal(
    <div
      ref={hostRef}
      role="tooltip"
      className="fixed z-[1000] max-h-[calc(100vh-8px)] max-w-80 overflow-y-auto rounded border border-border bg-popover px-2 py-2 text-[11px] leading-tight text-foreground shadow-md pointer-events-none"
      style={position}
    >
      <div className="text-base font-semibold leading-tight break-words mb-1">{common.message}</div>
      {active.type === 'range' ? (
        <dl className="grid grid-cols-[auto_1fr] gap-x-2 gap-y-1">
          <dt className="text-muted-foreground">duration</dt>
          <dd>
            {active.item.incomplete
              ? 'incomplete'
              : active.item.observed_duration === null
                ? 'unavailable'
                : formatNvtxDuration(active.item.observed_duration)}
          </dd>
          <dt className="text-muted-foreground">start</dt>
          <dd className="font-mono whitespace-nowrap">
            {formatNvtxDuration(active.item.observed_start)} relative to query
          </dd>
          <dt className="text-muted-foreground">end</dt>
          <dd className="font-mono whitespace-nowrap">
            {active.item.observed_end === null
              ? 'open at trace boundary'
              : `${formatNvtxDuration(active.item.observed_end)} relative to query`}
          </dd>
          <dt className="text-muted-foreground">kind</dt>
          <dd>{active.item.kind === 'push_pop' ? 'push/pop range' : 'start/end range'}</dd>
          {active.item.thread_name !== null && (
            <>
              <dt className="text-muted-foreground">thread</dt>
              <dd className="break-words">{active.item.thread_name}</dd>
            </>
          )}
          <dt className="text-muted-foreground">domain</dt>
          <dd className="break-words">{active.item.domain_name}</dd>
          <dt className="text-muted-foreground">category</dt>
          <dd className="break-words">{active.item.category_name ?? 'Uncategorized'}</dd>
        </dl>
      ) : (
        <dl className="grid grid-cols-[auto_1fr] gap-x-2 gap-y-1">
          <dt className="text-muted-foreground">timestamp</dt>
          <dd className="font-mono whitespace-nowrap">
            {formatNvtxDuration(active.item.timestamp)} relative to query
          </dd>
          <dt className="text-muted-foreground">domain</dt>
          <dd className="break-words">{active.item.domain_name}</dd>
          <dt className="text-muted-foreground">category</dt>
          <dd className="break-words">{active.item.category_name ?? 'Uncategorized'}</dd>
        </dl>
      )}
    </div>,
    document.body
  );
}

export interface NvtxLaneChartProps {
  lanes: NvtxLane[];
  durationSeconds: number;
  isDark: boolean;
}

export function NvtxLaneChart({ lanes, durationSeconds, isDark }: NvtxLaneChartProps) {
  const zoomRange = useZoomRange();
  const [active, setActive] = useState<ActiveItem | null>(null);
  const threadLanes = useMemo(() => lanes.filter(lane => lane.identity.kind === 'thread'), [lanes]);
  const maxDepth = useMemo(
    () =>
      threadLanes.reduce(
        (max, lane) => Math.max(max, lane.identity.kind === 'thread' ? lane.identity.depth : 0),
        0
      ),
    [threadLanes]
  );
  const laneBandCount = Math.max(maxDepth + 1, 1);
  const laneHeight = 18;
  const laneGap = 0;
  const verticalInset = 0;
  const contentHeight = laneBandCount * laneHeight + Math.max(0, laneBandCount - 1) * laneGap;
  const chartHeight = contentHeight;

  const groupedRanges = useMemo(
    () =>
      lanes.map((lane, laneIndex) => ({
        lane,
        laneIndex,
        depth: lane.identity.kind === 'thread' ? lane.identity.depth : 0,
        ranges: lane.ranges.map(range => ({
          range,
          startMs: nvtxRelativeSecondsToMs(range.display_start),
          endMs: nvtxRelativeSecondsToMs(range.display_end),
        })),
        marks: lane.marks.map(mark => ({
          mark,
          timestampMs: nvtxRelativeSecondsToMs(mark.timestamp),
        })),
      })),
    [lanes]
  );

  const data = useMemo<NvtxDatum[]>(
    () => [
      ...groupedRanges.flatMap(({ ranges, laneIndex, depth }) =>
        ranges.map(({ startMs, endMs }, itemIndex) => ({
          value: [startMs, endMs, depth] as [number, number, number],
          type: 'range' as const,
          laneIndex,
          itemIndex,
          depth,
        }))
      ),
      ...groupedRanges.flatMap(({ marks, laneIndex, depth }) =>
        marks.map(({ timestampMs }, itemIndex) => ({
          value: [timestampMs, timestampMs, depth] as [number, number, number],
          type: 'mark' as const,
          laneIndex,
          itemIndex,
          depth,
        }))
      ),
    ],
    [groupedRanges]
  );

  const renderItem: GanttRenderItem = useCallback(
    (params, api) => {
      const datum = data[params.dataIndex];
      if (!datum) return null;
      const startMs = api.value(0) as number;
      const endMs = api.value(1) as number;
      const startPoint = api.coord([startMs, datum.depth]);
      const endPoint = api.coord([endMs, datum.depth]);
      const coord = params.coordSys as { x?: number; y?: number; width?: number; height?: number };
      const clipBound =
        typeof coord.width === 'number' && typeof coord.height === 'number'
          ? { x: coord.x ?? 0, y: coord.y ?? 0, width: coord.width, height: coord.height }
          : null;

      if (datum.type === 'mark') {
        const mark = groupedRanges[datum.laneIndex]?.marks[datum.itemIndex]?.mark;
        if (!mark) return null;
        const x = startPoint[0];
        return {
          type: 'group' as const,
          children: [
            {
              type: 'line' as const,
              shape: {
                x1: x,
                y1: startPoint[1] - laneHeight / 2,
                x2: x,
                y2: startPoint[1] + laneHeight / 2,
              },
              style: { stroke: mark.color, lineWidth: 2 },
            },
            {
              type: 'polygon' as const,
              shape: {
                points: [
                  [x, startPoint[1] - laneHeight / 2],
                  [x - 4, startPoint[1] - laneHeight / 2 + 6],
                  [x + 4, startPoint[1] - laneHeight / 2 + 6],
                ],
              },
              style: { fill: mark.color },
            },
          ],
        };
      }

      const range = groupedRanges[datum.laneIndex]?.ranges[datum.itemIndex]?.range;
      if (!range) return null;
      const pixelWidth = endPoint[0] - startPoint[0];
      const renderedWidth = Math.max(2, pixelWidth);
      const rectShape = {
        x: pixelWidth < 2 ? startPoint[0] - renderedWidth / 2 : startPoint[0],
        y: startPoint[1] - laneHeight / 2,
        width: renderedWidth,
        height: laneHeight,
      };
      const clipped = clipBound ? clipRectByRect(rectShape, clipBound) : rectShape;
      if (!clipped) return null;
      const rect = {
        type: 'rect' as const,
        shape: { ...clipped, r: 2 },
        style: {
          fill: range.color,
          stroke: range.color,
          lineWidth: 1,
          opacity: range.incomplete ? 0.45 : 0.8,
        },
      };
      const edge = clipped.x + clipped.width;
      const openEdge = {
        type: 'line' as const,
        shape: { x1: edge, y1: clipped.y, x2: edge, y2: clipped.y + clipped.height },
        style: { stroke: range.color, lineWidth: 2, lineDash: [3, 2], opacity: 0.9 },
      };
      const label = range.message.trim();
      const text =
        clipped.width >= 36 && label.length > 0
          ? {
              type: 'text' as const,
              style: {
                x: clipped.x + 6,
                y: clipped.y + clipped.height / 2,
                text: label,
                fill: 'rgba(255,255,255,0.95)',
                fontSize: 11,
                fontWeight: 500,
                textVerticalAlign: 'middle' as const,
                textAlign: 'left' as const,
                width: Math.max(clipped.width - 12, 0),
                overflow: 'truncate' as const,
              },
              silent: true,
            }
          : null;
      return {
        type: 'group' as const,
        children: [rect, ...(range.incomplete ? [openEdge] : []), ...(text ? [text] : [])],
      };
    },
    [data, groupedRanges, laneHeight]
  );

  const zoomStartMs = zoomRange.start * 1_000;
  const zoomEndMs = zoomRange.end * 1_000;
  const zoomSpanMs = Math.max((zoomRange.end - zoomRange.start) * 1_000, 0.000001);
  const overlayStyle = (startMs: number, endMs: number, isMark: boolean) => {
    if (endMs < zoomStartMs || startMs > zoomEndMs) return { display: 'none' };
    const visibleStart = Math.max(startMs, zoomStartMs);
    const visibleEnd = Math.min(endMs, zoomEndMs);
    const left = ((visibleStart - zoomStartMs) / zoomSpanMs) * 100;
    const width = ((Math.max(visibleEnd, visibleStart) - visibleStart) / zoomSpanMs) * 100;
    if (isMark) return { left: `calc(${left}% - 6px)`, width: 12 };
    if (width < 0.15) return { left: `calc(${left}% - 1px)`, width: 2 };
    return { left: `${left}%`, width: `${width}%` };
  };

  const showAtElement = (
    type: ActiveItem['type'],
    item: NvtxRangeItem | NvtxMarkItem,
    element: HTMLElement
  ) => {
    const rect = element.getBoundingClientRect();
    const position = { x: rect.right, y: rect.top };
    setActive(
      type === 'range'
        ? { type, item: item as NvtxRangeItem, ...position }
        : { type, item: item as NvtxMarkItem, ...position }
    );
  };

  return (
    <div className="relative h-full min-h-11 overflow-hidden">
      <GanttChart
        data={data}
        gridSpacing={{ left: 0, right: 8, top: 0, bottom: 0 }}
        durationSeconds={durationSeconds}
        height={chartHeight}
        maxHeight={chartHeight}
        rowHeight={laneHeight}
        isDark={isDark}
        seriesName="nvtx-item"
        renderItem={renderItem}
        emptyMessage="No NVTX ranges in this view"
        cursor="pointer"
      />
      <div
        className="absolute inset-0 right-2 pointer-events-none"
        aria-label={`${lanes[0]?.label ?? 'NVTX'} NVTX items`}
      >
        {groupedRanges
          .flatMap(({ ranges, depth }) => ranges.map(entry => ({ ...entry, depth })))
          .map(({ range, startMs, endMs, depth }, index) => (
            <button
              key={`range-${index}`}
              type="button"
              className="absolute opacity-0 pointer-events-auto focus:opacity-100 focus:outline-2 focus:outline-primary rounded-sm"
              style={{
                ...overlayStyle(startMs, endMs, false),
                top: verticalInset + depth * (laneHeight + laneGap),
                height: laneHeight,
              }}
              aria-label={`${range.message}, ${range.incomplete ? 'incomplete' : 'NVTX range'}`}
              onPointerEnter={event => showAtElement('range', range, event.currentTarget)}
              onPointerLeave={() => setActive(null)}
              onFocus={event => showAtElement('range', range, event.currentTarget)}
              onBlur={() => setActive(null)}
              onKeyDown={event => event.key === 'Escape' && setActive(null)}
            />
          ))}
        {groupedRanges
          .flatMap(({ marks, depth }) => marks.map(entry => ({ ...entry, depth })))
          .map(({ mark, timestampMs, depth }, index) => (
            <button
              key={`mark-${index}`}
              type="button"
              className="absolute opacity-0 pointer-events-auto focus:opacity-100 focus:outline-2 focus:outline-primary rounded-sm"
              style={{
                ...overlayStyle(timestampMs, timestampMs, true),
                top: verticalInset + depth * (laneHeight + laneGap),
                height: laneHeight,
              }}
              aria-label={`${mark.message}, NVTX mark`}
              onPointerEnter={event => showAtElement('mark', mark, event.currentTarget)}
              onPointerLeave={() => setActive(null)}
              onFocus={event => showAtElement('mark', mark, event.currentTarget)}
              onBlur={() => setActive(null)}
              onKeyDown={event => event.key === 'Escape' && setActive(null)}
            />
          ))}
      </div>
      {active && <NvtxItemTooltip active={active} />}
    </div>
  );
}
