// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from 'vitest';
import type { QueryBundle, EntityRef, Operator, PlanTree } from '@quent/utils';
import {
  clipRectByRect,
  operatorTimelineRowId,
  workerIdFromOperatorTimelineRowId,
  getWorkerIdsFromPlanTree,
  getPlanIdsForWorker,
  stackOperatorsIntoRows,
  spanToMs,
  operatorsWithActiveSpans,
  operatorsWithActiveSpansForWorker,
} from './utils';

// ---- Helpers ---------------------------------------------------------------

function makeOp(overrides: Partial<Operator> = {}): Operator {
  return {
    id: 'op',
    plan_id: null,
    parent_operator_ids: [],
    instance_name: null,
    operator_type_name: null,
    custom_attributes: {},
    statistics: null,
    active_span: null,
    ...overrides,
  };
}

function makeBundle(
  operators: Record<string, Operator | null | undefined>,
  planTree: PlanTree = { id: 'root', worker: null, children: [] }
): QueryBundle<EntityRef> {
  return {
    entities: { operators },
  } as unknown as QueryBundle<EntityRef>;
  // plan_tree only used by operatorsWithActiveSpansForWorker — provided inline there
  void planTree;
}

function makeBundleWithTree(
  operators: Record<string, Operator | null | undefined>,
  planTree: PlanTree
): QueryBundle<EntityRef> {
  return {
    entities: { operators },
    plan_tree: planTree,
  } as unknown as QueryBundle<EntityRef>;
}

type Rect = { x: number; y: number; width: number; height: number };
function rect(x: number, y: number, width: number, height: number): Rect {
  return { x, y, width, height };
}

// ---- clipRectByRect --------------------------------------------------------

describe('clipRectByRect', () => {
  const bounds = rect(10, 10, 100, 100);

  it('returns the target when it is fully inside bounds', () => {
    const target = rect(20, 20, 30, 30);
    expect(clipRectByRect(target, bounds)).toEqual(target);
  });

  it('returns undefined when target is fully to the left of bounds', () => {
    expect(clipRectByRect(rect(0, 20, 5, 10), bounds)).toBeUndefined();
  });

  it('returns undefined when target is fully to the right of bounds', () => {
    expect(clipRectByRect(rect(120, 20, 10, 10), bounds)).toBeUndefined();
  });

  it('returns undefined when target is fully above bounds', () => {
    expect(clipRectByRect(rect(20, 0, 10, 5), bounds)).toBeUndefined();
  });

  it('returns undefined when target is fully below bounds', () => {
    expect(clipRectByRect(rect(20, 120, 10, 10), bounds)).toBeUndefined();
  });

  it('clips the left edge', () => {
    // target starts at x=5, extends to x=50 → clipped to x=10..50
    const result = clipRectByRect(rect(5, 20, 45, 10), bounds);
    expect(result).toEqual(rect(10, 20, 40, 10));
  });

  it('clips the right edge', () => {
    // target starts at x=80, extends to x=120 → clipped to x=80..110
    const result = clipRectByRect(rect(80, 20, 40, 10), bounds);
    expect(result).toEqual(rect(80, 20, 30, 10));
  });

  it('clips the top edge', () => {
    const result = clipRectByRect(rect(20, 5, 10, 20), bounds);
    expect(result).toEqual(rect(20, 10, 10, 15));
  });

  it('clips the bottom edge', () => {
    const result = clipRectByRect(rect(20, 90, 10, 40), bounds);
    expect(result).toEqual(rect(20, 90, 10, 20));
  });

  it('clips a target larger than bounds in all directions', () => {
    const result = clipRectByRect(rect(0, 0, 200, 200), bounds);
    expect(result).toEqual(bounds);
  });

  it('includes a zero-width result when target right edge exactly touches bounds left edge', () => {
    // target: x=0..10 → x2 = min(10, 110) = 10, x = max(0,10) = 10 → width = 0
    const result = clipRectByRect(rect(0, 20, 10, 10), bounds);
    expect(result).toEqual(rect(10, 20, 0, 10));
  });
});

// ---- operatorTimelineRowId / workerIdFromOperatorTimelineRowId -------------

describe('operatorTimelineRowId', () => {
  it('produces a stable prefixed id', () => {
    expect(operatorTimelineRowId('worker-42')).toBe('__operator_timeline__worker-42');
  });

  it('round-trips through workerIdFromOperatorTimelineRowId', () => {
    const id = operatorTimelineRowId('w1');
    expect(workerIdFromOperatorTimelineRowId(id)).toBe('w1');
  });
});

describe('workerIdFromOperatorTimelineRowId', () => {
  it('extracts the worker id from a valid row id', () => {
    expect(workerIdFromOperatorTimelineRowId('__operator_timeline__w99')).toBe('w99');
  });

  it('returns null for an id without the prefix', () => {
    expect(workerIdFromOperatorTimelineRowId('some-resource-id')).toBeNull();
  });

  it('returns null for an empty string', () => {
    expect(workerIdFromOperatorTimelineRowId('')).toBeNull();
  });
});

// ---- getWorkerIdsFromPlanTree -----------------------------------------------

describe('getWorkerIdsFromPlanTree', () => {
  it('returns an empty array for a root with no worker and no children', () => {
    expect(getWorkerIdsFromPlanTree({ id: 'root', worker: null, children: [] })).toEqual([]);
  });

  it('returns the root worker when set', () => {
    expect(getWorkerIdsFromPlanTree({ id: 'r', worker: 'w1', children: [] })).toEqual(['w1']);
  });

  it('skips empty-string worker values', () => {
    expect(getWorkerIdsFromPlanTree({ id: 'r', worker: '', children: [] })).toEqual([]);
  });

  it('collects workers from nested children', () => {
    const tree: PlanTree = {
      id: 'root',
      worker: null,
      children: [
        { id: 'p1', worker: 'w1', children: [] },
        { id: 'p2', worker: 'w2', children: [{ id: 'p3', worker: 'w3', children: [] }] },
      ],
    };
    const result = getWorkerIdsFromPlanTree(tree);
    expect(result.sort()).toEqual(['w1', 'w2', 'w3']);
  });

  it('deduplicates workers that appear in multiple nodes', () => {
    const tree: PlanTree = {
      id: 'root',
      worker: 'w1',
      children: [{ id: 'p1', worker: 'w1', children: [] }],
    };
    expect(getWorkerIdsFromPlanTree(tree)).toEqual(['w1']);
  });
});

// ---- getPlanIdsForWorker ----------------------------------------------------

describe('getPlanIdsForWorker', () => {
  const tree: PlanTree = {
    id: 'root',
    worker: null,
    children: [
      { id: 'p1', worker: 'w1', children: [] },
      {
        id: 'p2',
        worker: 'w2',
        children: [{ id: 'p3', worker: 'w1', children: [] }],
      },
    ],
  };

  it('returns an empty array when no node matches the workerId', () => {
    expect(getPlanIdsForWorker(tree, 'w99')).toEqual([]);
  });

  it('finds a single direct child node', () => {
    expect(getPlanIdsForWorker(tree, 'w2')).toEqual(['p2']);
  });

  it('collects all nodes (including nested) matching the workerId', () => {
    const result = getPlanIdsForWorker(tree, 'w1');
    expect(result).toContain('p1');
    expect(result).toContain('p3');
    expect(result).toHaveLength(2);
  });

  it('returns an empty array for a leaf tree with no matching worker', () => {
    const leaf: PlanTree = { id: 'leaf', worker: null, children: [] };
    expect(getPlanIdsForWorker(leaf, 'w1')).toEqual([]);
  });
});

// ---- stackOperatorsIntoRows ------------------------------------------------

type Span = { startMs: number; endMs: number; rowIndex: number };

function span(startMs: number, endMs: number): Span {
  return { startMs, endMs, rowIndex: 0 };
}

describe('stackOperatorsIntoRows', () => {
  it('returns an empty array unchanged', () => {
    expect(stackOperatorsIntoRows([])).toEqual([]);
  });

  it('places a single entry in row 0', () => {
    const [entry] = stackOperatorsIntoRows([span(0, 10)]);
    expect(entry.rowIndex).toBe(0);
  });

  it('places non-overlapping entries in row 0', () => {
    const entries = [span(0, 10), span(20, 30), span(40, 50)];
    stackOperatorsIntoRows(entries);
    expect(entries.map(e => e.rowIndex)).toEqual([0, 0, 0]);
  });

  it('separates fully overlapping entries into different rows', () => {
    const a = span(0, 10);
    const b = span(0, 10);
    stackOperatorsIntoRows([a, b]);
    expect(a.rowIndex).not.toBe(b.rowIndex);
  });

  it('allows a bar that starts exactly where the previous one ends to share the same row', () => {
    // A=[0,10], B=[10,20]: B.startMs < rowEndMs[0] → 10 < 10 is false → row 0
    const a = span(0, 10);
    const b = span(10, 20);
    stackOperatorsIntoRows([a, b]);
    expect(a.rowIndex).toBe(0);
    expect(b.rowIndex).toBe(0);
  });

  it('packs overlapping bars into the minimum number of rows', () => {
    // A=[0,10], B=[5,15], C=[12,20]: C fits in row 0 after A
    const a = span(0, 10);
    const b = span(5, 15);
    const c = span(12, 20);
    stackOperatorsIntoRows([a, b, c]);
    expect(a.rowIndex).toBe(0);
    expect(b.rowIndex).toBe(1);
    expect(c.rowIndex).toBe(0);
  });

  it('uses three rows when three bars all overlap', () => {
    const a = span(0, 30);
    const b = span(5, 20);
    const c = span(10, 15);
    stackOperatorsIntoRows([a, b, c]);
    const rows = new Set([a.rowIndex, b.rowIndex, c.rowIndex]);
    expect(rows.size).toBe(3);
  });

  it('mutates the original entries and returns the same array reference', () => {
    const entries = [span(0, 10), span(5, 20)];
    const result = stackOperatorsIntoRows(entries);
    expect(result).toBe(entries);
    expect(entries[0].rowIndex).toBeDefined();
  });

  it('handles entries provided in reverse order (sorts by startMs internally)', () => {
    const a = span(10, 20);
    const b = span(0, 5);
    stackOperatorsIntoRows([a, b]);
    // b starts first, both fit in row 0
    expect(a.rowIndex).toBe(0);
    expect(b.rowIndex).toBe(0);
  });
});

// ---- spanToMs --------------------------------------------------------------

describe('spanToMs', () => {
  it('converts a zero span at epoch zero to {startMs: 0, endMs: 0}', () => {
    expect(spanToMs({ start: 0, end: 0 }, 0n)).toEqual({ startMs: 0, endMs: 0 });
  });

  it('converts span seconds to milliseconds offset from startTimeNs', () => {
    // startTimeNs = 1ms = 1_000_000n, span = {start: 1s, end: 2s}
    expect(spanToMs({ start: 1, end: 2 }, 1_000_000n)).toEqual({
      startMs: 1001,
      endMs: 2001,
    });
  });

  it('handles fractional seconds in the span', () => {
    // startTimeNs = 0, span = {start: 0.5s, end: 1.5s}
    expect(spanToMs({ start: 0.5, end: 1.5 }, 0n)).toEqual({
      startMs: 500,
      endMs: 1500,
    });
  });

  it('handles a large epoch startTimeNs', () => {
    // 1 second = 1_000_000_000n ns → 1000ms; span adds 0
    expect(spanToMs({ start: 0, end: 0 }, 1_000_000_000n)).toEqual({
      startMs: 1000,
      endMs: 1000,
    });
  });
});

// ---- operatorsWithActiveSpans ----------------------------------------------

describe('operatorsWithActiveSpans', () => {
  it('returns [] when operators is absent', () => {
    const bundle = { entities: {} } as unknown as QueryBundle<EntityRef>;
    expect(operatorsWithActiveSpans(bundle, 0n, 'p1')).toEqual([]);
  });

  it('returns [] when planId is null', () => {
    const bundle = makeBundle({
      op1: makeOp({ plan_id: 'p1', active_span: { start: 0, end: 1 } }),
    });
    expect(operatorsWithActiveSpans(bundle, 0n, null)).toEqual([]);
  });

  it('returns [] when planId is empty string', () => {
    const bundle = makeBundle({
      op1: makeOp({ plan_id: 'p1', active_span: { start: 0, end: 1 } }),
    });
    expect(operatorsWithActiveSpans(bundle, 0n, '')).toEqual([]);
  });

  it('returns [] when no operator has a matching plan_id', () => {
    const bundle = makeBundle({
      op1: makeOp({ plan_id: 'p2', active_span: { start: 0, end: 1 } }),
    });
    expect(operatorsWithActiveSpans(bundle, 0n, 'p1')).toEqual([]);
  });

  it('filters out operators without an active_span', () => {
    const bundle = makeBundle({
      op1: makeOp({ id: 'op1', plan_id: 'p1', active_span: null }),
    });
    expect(operatorsWithActiveSpans(bundle, 0n, 'p1')).toEqual([]);
  });

  it('returns an entry for a matching operator with an active_span', () => {
    const bundle = makeBundle({
      op1: makeOp({
        id: 'op1',
        plan_id: 'p1',
        active_span: { start: 0, end: 1 },
        operator_type_name: 'Scan',
        instance_name: 'my-scan',
      }),
    });
    const result = operatorsWithActiveSpans(bundle, 0n, 'p1');
    expect(result).toHaveLength(1);
    expect(result[0]).toMatchObject({
      operatorId: 'op1',
      label: 'my-scan',
      typeName: 'Scan',
      planId: 'p1',
      startMs: 0,
      endMs: 1000,
      rowIndex: 0,
    });
  });

  it('falls back to operator_type_name when instance_name is null', () => {
    const bundle = makeBundle({
      op1: makeOp({
        id: 'op1',
        plan_id: 'p1',
        active_span: { start: 0, end: 1 },
        operator_type_name: 'Join',
        instance_name: null,
      }),
    });
    const [entry] = operatorsWithActiveSpans(bundle, 0n, 'p1');
    expect(entry.label).toBe('Join');
  });

  it('filters out null entries in the operators map', () => {
    const bundle = makeBundle({ op1: null });
    expect(operatorsWithActiveSpans(bundle, 0n, 'p1')).toEqual([]);
  });

  it('stacks overlapping operators into multiple rows', () => {
    const bundle = makeBundle({
      op1: makeOp({ id: 'op1', plan_id: 'p1', active_span: { start: 0, end: 10 } }),
      op2: makeOp({ id: 'op2', plan_id: 'p1', active_span: { start: 2, end: 8 } }),
    });
    const result = operatorsWithActiveSpans(bundle, 0n, 'p1');
    expect(result).toHaveLength(2);
    const rows = new Set(result.map(e => e.rowIndex));
    expect(rows.size).toBe(2);
  });
});

// ---- operatorsWithActiveSpansForWorker -------------------------------------

describe('operatorsWithActiveSpansForWorker', () => {
  it('returns [] when operators is absent', () => {
    const bundle = {
      entities: {},
      plan_tree: { id: 'root', worker: null, children: [] },
    } as unknown as QueryBundle<EntityRef>;
    expect(operatorsWithActiveSpansForWorker(bundle, 0n, 'w1')).toEqual([]);
  });

  it('returns [] when no plan belongs to the worker', () => {
    const bundle = makeBundleWithTree(
      { op1: makeOp({ plan_id: 'p1', active_span: { start: 0, end: 1 } }) },
      { id: 'root', worker: 'w2', children: [] }
    );
    expect(operatorsWithActiveSpansForWorker(bundle, 0n, 'w1')).toEqual([]);
  });

  it('returns entries for operators whose plan belongs to the worker', () => {
    const planTree: PlanTree = {
      id: 'root',
      worker: null,
      children: [{ id: 'p1', worker: 'w1', children: [] }],
    };
    const bundle = makeBundleWithTree(
      {
        op1: makeOp({
          id: 'op1',
          plan_id: 'p1',
          active_span: { start: 0, end: 2 },
          operator_type_name: 'Scan',
        }),
      },
      planTree
    );
    const result = operatorsWithActiveSpansForWorker(bundle, 0n, 'w1');
    expect(result).toHaveLength(1);
    expect(result[0].operatorId).toBe('op1');
    expect(result[0].planId).toBe('p1');
  });

  it('includes operators from multiple plans belonging to the same worker', () => {
    const planTree: PlanTree = {
      id: 'root',
      worker: null,
      children: [
        { id: 'p1', worker: 'w1', children: [] },
        { id: 'p2', worker: 'w1', children: [] },
      ],
    };
    const bundle = makeBundleWithTree(
      {
        op1: makeOp({ id: 'op1', plan_id: 'p1', active_span: { start: 0, end: 1 } }),
        op2: makeOp({ id: 'op2', plan_id: 'p2', active_span: { start: 2, end: 3 } }),
      },
      planTree
    );
    const result = operatorsWithActiveSpansForWorker(bundle, 0n, 'w1');
    expect(result).toHaveLength(2);
  });

  it('excludes operators belonging to a different worker plan', () => {
    const planTree: PlanTree = {
      id: 'root',
      worker: null,
      children: [
        { id: 'p1', worker: 'w1', children: [] },
        { id: 'p2', worker: 'w2', children: [] },
      ],
    };
    const bundle = makeBundleWithTree(
      {
        op1: makeOp({ id: 'op1', plan_id: 'p1', active_span: { start: 0, end: 1 } }),
        op2: makeOp({ id: 'op2', plan_id: 'p2', active_span: { start: 0, end: 1 } }),
      },
      planTree
    );
    const result = operatorsWithActiveSpansForWorker(bundle, 0n, 'w1');
    expect(result).toHaveLength(1);
    expect(result[0].operatorId).toBe('op1');
  });

  it('filters out operators without active_span', () => {
    const planTree: PlanTree = {
      id: 'p1',
      worker: 'w1',
      children: [],
    };
    const bundle = makeBundleWithTree(
      { op1: makeOp({ plan_id: 'p1', active_span: null }) },
      planTree
    );
    expect(operatorsWithActiveSpansForWorker(bundle, 0n, 'w1')).toEqual([]);
  });
});
