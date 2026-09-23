// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createStore } from 'jotai';
import { describe, expect, it } from 'vitest';
import {
  dagHeatmapRangeAtom,
  dagDisplayedNodeIdsAtom,
  dagNodeGroupsAtom,
  hoveredStatAtom,
} from './dagControls';

describe('dagHeatmapRangeAtom', () => {
  it('is null when no stat is hovered', () => {
    const store = createStore();
    expect(store.get(dagHeatmapRangeAtom)).toBeNull();
  });

  it('ranges over every displayed node’s resolved value, mixing direct and aggregated', () => {
    const store = createStore();
    // Physical operators a..d each have a small value. 'plain' is a node with
    // no related operators, so it resolves directly against its own entry.
    // Two grouped nodes sum different physical operators, each summing well
    // past any single item's value.
    store.set(hoveredStatAtom, {
      name: 'duration_s',
      values: new Map([
        ['plain', 1],
        ['a', 2],
        ['b', 3],
        ['c', 4],
        ['d', 5],
      ]),
      min: 1,
      max: 5,
      aggMode: 'sum',
    });
    store.set(
      dagDisplayedNodeIdsAtom,
      new Set(['plain', 'grouped-1', 'grouped-2', 'a', 'b', 'c', 'd'])
    );
    store.set(
      dagNodeGroupsAtom,
      new Map([
        ['plain', []], // direct value: 1
        ['grouped-1', ['a', 'b']], // sums to 5
        ['grouped-2', ['c', 'd']], // sums to 9
      ])
    );

    // The direct value (1, from 'plain') and the largest aggregated value (9,
    // from 'grouped-2') both bound the range, so a plain operator and a
    // grouped node are colored on the same scale.
    expect(store.get(dagHeatmapRangeAtom)).toEqual({ min: 1, max: 9 });
  });

  it('is null when nothing in the displayed DAG resolves to a value', () => {
    const store = createStore();
    store.set(hoveredStatAtom, {
      name: 'duration_s',
      values: new Map([['unrelated', 1]]),
      min: 1,
      max: 1,
      aggMode: 'sum',
    });
    store.set(dagDisplayedNodeIdsAtom, new Set(['a']));
    store.set(dagNodeGroupsAtom, new Map([['a', []]]));

    expect(store.get(dagHeatmapRangeAtom)).toBeNull();
  });
});
