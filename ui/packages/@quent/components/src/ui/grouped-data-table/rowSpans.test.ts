// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { computeRowSpans } from './rowSpans';

describe('computeRowSpans', () => {
  it('returns an empty array for empty rows', () => {
    expect(computeRowSpans([])).toEqual([]);
  });

  it('assigns span=1 to every row when all group ids are distinct', () => {
    const rows = [
      { groupKeys: [{ id: 'a' }] },
      { groupKeys: [{ id: 'b' }] },
      { groupKeys: [{ id: 'c' }] },
    ];
    expect(computeRowSpans(rows)).toEqual([[1], [1], [1]]);
  });

  it('merges consecutive same-id rows in the outer column', () => {
    const rows = [
      { groupKeys: [{ id: 'a' }, { id: 'x' }] },
      { groupKeys: [{ id: 'a' }, { id: 'y' }] },
      { groupKeys: [{ id: 'b' }, { id: 'z' }] },
    ];
    const spans = computeRowSpans(rows);
    // col 0: 'a' spans rows 0-1, 'b' spans row 2
    expect(spans[0]![0]).toBe(2);
    expect(spans[1]![0]).toBeNull();
    expect(spans[2]![0]).toBe(1);
  });

  it('merges a column when both the id and its parent are the same', () => {
    // rows 0 and 1 share the same (col0, col1) pair
    const rows = [
      { groupKeys: [{ id: 'a' }, { id: 'x' }] },
      { groupKeys: [{ id: 'a' }, { id: 'x' }] },
      { groupKeys: [{ id: 'b' }, { id: 'z' }] },
    ];
    const spans = computeRowSpans(rows);
    expect(spans[0]![0]).toBe(2); // 'a' spans 2 rows
    expect(spans[0]![1]).toBe(2); // 'x' under 'a' also spans 2
    expect(spans[1]![0]).toBeNull();
    expect(spans[1]![1]).toBeNull();
  });

  it('splits inner spans when the parent group changes', () => {
    // 'x' appears under both 'a' and 'b' — must not be merged across parents
    const rows = [
      { groupKeys: [{ id: 'a' }, { id: 'x' }] },
      { groupKeys: [{ id: 'b' }, { id: 'x' }] },
    ];
    const spans = computeRowSpans(rows);
    expect(spans[0]![1]).toBe(1);
    expect(spans[1]![1]).toBe(1);
  });
});
