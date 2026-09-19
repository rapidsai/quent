// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createStore } from 'jotai';
import { describe, expect, it } from 'vitest';
import {
  dagAggregatedHeatmapRangeAtom,
  dagDisplayedNodeIdsAtom,
  dagNodeGroupsAtom,
  hoveredStatAtom,
} from './dagControls';

describe('dagAggregatedHeatmapRangeAtom', () => {
  it('is null when no stat is hovered', () => {
    const store = createStore();
    expect(store.get(dagAggregatedHeatmapRangeAtom)).toBeNull();
  });

  it('ranges over aggregated (grouped) node values, not raw per-item values', () => {
    const store = createStore();
    // Physical operators a..d each have a small value; the table's raw
    // min/max reflects only those. Two logical nodes group different
    // physical operators, each summing well past the raw item max.
    store.set(hoveredStatAtom, {
      name: 'duration_s',
      values: new Map([
        ['a', 1],
        ['b', 2],
        ['c', 3],
        ['d', 4],
      ]),
      min: 1,
      max: 4,
      aggMode: 'sum',
    });
    store.set(dagDisplayedNodeIdsAtom, new Set(['logical-1', 'logical-2', 'a', 'b', 'c', 'd']));
    store.set(
      dagNodeGroupsAtom,
      new Map([
        ['logical-1', ['a', 'b']], // sums to 3
        ['logical-2', ['c', 'd']], // sums to 7
      ])
    );

    expect(store.get(dagAggregatedHeatmapRangeAtom)).toEqual({ min: 3, max: 7 });
  });

  it('ignores nodes with no related operators (they use the raw range instead)', () => {
    const store = createStore();
    store.set(hoveredStatAtom, {
      name: 'duration_s',
      values: new Map([['a', 1]]),
      min: 1,
      max: 1,
      aggMode: 'sum',
    });
    store.set(dagDisplayedNodeIdsAtom, new Set(['a']));
    store.set(dagNodeGroupsAtom, new Map([['a', []]]));

    expect(store.get(dagAggregatedHeatmapRangeAtom)).toBeNull();
  });
});
