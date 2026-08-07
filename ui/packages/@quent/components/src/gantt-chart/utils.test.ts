// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { clipRectByRect, stackIntervalsIntoRows, type GanttRect } from './utils';

function rect(x: number, y: number, width: number, height: number): GanttRect {
  return { x, y, width, height };
}

describe('clipRectByRect', () => {
  const bounds = rect(10, 10, 100, 100);

  it('returns the target when it is fully inside bounds', () => {
    const target = rect(20, 20, 30, 30);
    expect(clipRectByRect(target, bounds)).toEqual(target);
  });

  it('returns undefined when the target is outside bounds', () => {
    expect(clipRectByRect(rect(0, 20, 5, 10), bounds)).toBeUndefined();
    expect(clipRectByRect(rect(120, 20, 10, 10), bounds)).toBeUndefined();
    expect(clipRectByRect(rect(20, 0, 10, 5), bounds)).toBeUndefined();
    expect(clipRectByRect(rect(20, 120, 10, 10), bounds)).toBeUndefined();
  });

  it('clips each edge to the bounds', () => {
    expect(clipRectByRect(rect(5, 20, 45, 10), bounds)).toEqual(rect(10, 20, 40, 10));
    expect(clipRectByRect(rect(80, 20, 40, 10), bounds)).toEqual(rect(80, 20, 30, 10));
    expect(clipRectByRect(rect(20, 5, 10, 20), bounds)).toEqual(rect(20, 10, 10, 15));
    expect(clipRectByRect(rect(20, 90, 10, 40), bounds)).toEqual(rect(20, 90, 10, 20));
  });

  it('clips a target larger than bounds in every direction', () => {
    expect(clipRectByRect(rect(0, 0, 200, 200), bounds)).toEqual(bounds);
  });

  it('includes a zero-width result when edges touch', () => {
    expect(clipRectByRect(rect(0, 20, 10, 10), bounds)).toEqual(rect(10, 20, 0, 10));
  });
});

type Span = { startMs: number; endMs: number; rowIndex: number };

function span(startMs: number, endMs: number): Span {
  return { startMs, endMs, rowIndex: 0 };
}

describe('stackIntervalsIntoRows', () => {
  it('returns an empty array unchanged', () => {
    expect(stackIntervalsIntoRows([])).toEqual([]);
  });

  it('packs adjacent and non-overlapping intervals into one row', () => {
    const entries = [span(0, 10), span(10, 20), span(40, 50)];
    stackIntervalsIntoRows(entries);
    expect(entries.map(entry => entry.rowIndex)).toEqual([0, 0, 0]);
  });

  it('reuses the first compatible row', () => {
    const a = span(0, 10);
    const b = span(5, 15);
    const c = span(12, 20);
    stackIntervalsIntoRows([a, b, c]);
    expect([a.rowIndex, b.rowIndex, c.rowIndex]).toEqual([0, 1, 0]);
  });

  it('uses input order as the packing priority', () => {
    const rankedFirst = span(5, 10);
    const rankedSecond = span(0, 6);
    const entries = [rankedFirst, rankedSecond];
    expect(stackIntervalsIntoRows(entries)).toBe(entries);
    expect(entries.map(entry => entry.rowIndex)).toEqual([0, 1]);
  });

  it('does not move existing entries when new entries are appended', () => {
    const existing = [span(5, 10), span(0, 6), span(10, 20)];
    stackIntervalsIntoRows(existing);
    const previousRows = existing.map(entry => entry.rowIndex);

    stackIntervalsIntoRows([...existing, span(4, 12)]);

    expect(existing.map(entry => entry.rowIndex)).toEqual(previousRows);
  });
});
