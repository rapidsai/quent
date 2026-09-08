// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { TIMELINE_SPACING } from '../timeline/types';
import type { GanttGridSpacing, GanttRenderItem } from './options';

export interface GanttRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

type GanttRenderItemParams = Parameters<GanttRenderItem>[0];
type GanttRenderItemApi = Parameters<GanttRenderItem>[1];

/** Height reserved for the expand/collapse control. */
export const GANTT_RESIZE_CONTROL_HEIGHT = 12;

/** Clip a rectangle to the chart grid bounds. */
export function clipRectByRect(target: GanttRect, bounds: GanttRect): GanttRect | undefined {
  const x = Math.max(target.x, bounds.x);
  const x2 = Math.min(target.x + target.width, bounds.x + bounds.width);
  const y = Math.max(target.y, bounds.y);
  const y2 = Math.min(target.y + target.height, bounds.y + bounds.height);
  if (x2 >= x && y2 >= y) {
    return { x, y, width: x2 - x, height: y2 - y };
  }
  return undefined;
}

type PackedInterval = { startMs: number; endMs: number };

function findInsertionIndex(intervals: PackedInterval[], startMs: number): number {
  let low = 0;
  let high = intervals.length;
  while (low < high) {
    const mid = Math.floor((low + high) / 2);
    if (intervals[mid]!.startMs < startMs) {
      low = mid + 1;
    } else {
      high = mid;
    }
  }
  return low;
}

/** Greedily pack intervals in input order so appended entries do not move existing rows. */
export function stackIntervalsIntoRows<
  T extends { startMs: number; endMs: number; rowIndex: number },
>(entries: readonly T[]): T[] {
  const rows: PackedInterval[][] = [];
  const stackedEntries: T[] = [];
  for (const entry of entries) {
    let row = 0;
    let insertionIndex = 0;
    while (row < rows.length) {
      const intervals = rows[row]!;
      insertionIndex = findInsertionIndex(intervals, entry.startMs);
      const previous = intervals[insertionIndex - 1];
      const next = intervals[insertionIndex];
      if (
        (previous == null || previous.endMs <= entry.startMs) &&
        (next == null || entry.endMs <= next.startMs)
      ) {
        break;
      }
      row++;
    }

    if (row === rows.length) {
      rows.push([]);
    }
    const stackedEntry = { ...entry, rowIndex: row };
    rows[row]!.splice(insertionIndex, 0, stackedEntry);
    stackedEntries.push(stackedEntry);
  }

  return stackedEntries;
}

/** Grid clip rect from an ECharts custom-series `coordSys`, or `null` if unknown. */
export function ganttClipBound(coordSys: unknown): GanttRect | null {
  const coord = coordSys as { x?: number; y?: number; width?: number; height?: number };
  if (typeof coord.width !== 'number' || typeof coord.height !== 'number') {
    return null;
  }
  return { x: coord.x ?? 0, y: coord.y ?? 0, width: coord.width, height: coord.height };
}

/** Pixel rect for a Gantt bar centered on the category tick. */
export function ganttBarShape(
  startPoint: readonly [number, number],
  endPoint: readonly [number, number],
  barHeight: number,
  minWidth = 1
): GanttRect {
  return {
    x: startPoint[0],
    y: startPoint[1] - barHeight / 2,
    width: Math.max(minWidth, endPoint[0] - startPoint[0]),
    height: barHeight,
  };
}

/** Shared custom-series layout: value → clipped bar, used by operator/entity/NVTX Gantts. */
export function layoutGanttBar(
  params: GanttRenderItemParams,
  api: GanttRenderItemApi,
  options: { barHeight: number; minWidth?: number; allowInstant?: boolean }
): { startMs: number; endMs: number; rowIndex: number; clippedShape: GanttRect } | null {
  const startMs = api.value(0) as number;
  const endMs = api.value(1) as number;
  const rowIndex = api.value(2) as number;
  if (endMs < startMs) {
    return null;
  }
  if (endMs === startMs && !options.allowInstant) {
    return null;
  }
  const startPoint = api.coord([startMs, rowIndex]) as [number, number];
  const endPoint = api.coord([endMs, rowIndex]) as [number, number];
  const rectShape = ganttBarShape(startPoint, endPoint, options.barHeight, options.minWidth ?? 1);
  const clipBound = ganttClipBound(params.coordSys);
  const clippedShape = clipBound ? clipRectByRect(rectShape, clipBound) : rectShape;
  if (!clippedShape) {
    return null;
  }
  return { startMs, endMs, rowIndex, clippedShape };
}

/** Collapsed vs expanded max-height for a stacked Gantt, matching the long-entities control. */
export function ganttExpansionLayout({
  rowCount,
  rowHeight,
  collapsedHeight,
  isExpanded,
}: {
  rowCount: number;
  rowHeight: number;
  collapsedHeight: number;
  isExpanded: boolean;
}): {
  canResize: boolean;
  resizeControlHeight: number;
  contentHeight: number;
  maxHeight: number;
  contentPaddingBottom: number;
  gridSpacing: GanttGridSpacing;
} {
  const rowsHeight = rowCount * rowHeight + TIMELINE_SPACING.top + TIMELINE_SPACING.bottom;
  const canResize = rowsHeight > collapsedHeight;
  const resizeControlHeight = canResize ? GANTT_RESIZE_CONTROL_HEIGHT : 0;
  const contentHeight = Math.max(collapsedHeight, rowsHeight + resizeControlHeight);
  return {
    canResize,
    resizeControlHeight,
    contentHeight,
    maxHeight: isExpanded ? contentHeight : collapsedHeight,
    contentPaddingBottom: resizeControlHeight,
    gridSpacing: {
      ...TIMELINE_SPACING,
      bottom: TIMELINE_SPACING.bottom + resizeControlHeight,
    },
  };
}
