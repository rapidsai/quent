// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

export interface GanttRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

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
    if (intervals[mid]!.startMs < startMs) low = mid + 1;
    else high = mid;
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

    if (row === rows.length) rows.push([]);
    const stackedEntry = { ...entry, rowIndex: row };
    rows[row]!.splice(insertionIndex, 0, stackedEntry);
    stackedEntries.push(stackedEntry);
  }

  return stackedEntries;
}
