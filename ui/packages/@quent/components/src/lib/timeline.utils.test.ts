// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect, vi } from 'vitest';

// Prevent ECharts canvas initialization in jsdom
vi.mock('../lib/echarts', () => ({
  connect: vi.fn(),
  disconnect: vi.fn(),
  getInstanceByDom: vi.fn(),
}));

import {
  nanosToMs,
  getLongEntitiesThreshold,
  getTimelineXAxisIntervalMs,
  dimSeries,
  mergeOverlaySeries,
  setOperatorOnEntry,
  setOperatorOnEntries,
  findItemById,
  computeVisibleMaxValue,
} from './timeline.utils';
import type { TimelineSeries, TimelineSeriesEntry } from '../timeline/types';
import type { TreeTableItem } from '../resource-tree/types';
import type { TimelineRequest, OperatorFilter } from '@quent/utils';

// ---- Helpers ---------------------------------------------------------------

function makeEntry(color: string, overrides?: Partial<TimelineSeriesEntry>): TimelineSeriesEntry {
  return {
    color,
    formatter: (v: number) => String(v),
    values: [],
    binDuration: 1000,
    ...overrides,
  };
}

function makeTreeItem(id: string, children?: TreeTableItem[]): TreeTableItem {
  return {
    id,
    type: 'Resource',
    entity: null as never,
    children,
  };
}

const baseConfig = { start: 0, end: 10, num_bins: 10 };
const baseFilter = { entity_type_name: null };

function makeResourceEntry(): TimelineRequest<OperatorFilter> {
  return {
    Resource: {
      resource_id: 'r1',
      long_entities_threshold_s: null,
      entity_filter: baseFilter,
      application: { operator_ids: [] },
      config: baseConfig,
    },
  };
}

function makeGroupEntry(): TimelineRequest<OperatorFilter> {
  return {
    ResourceGroup: {
      resource_group_id: 'g1',
      resource_type_name: 'disk',
      long_entities_threshold_s: null,
      entity_filter: baseFilter,
      app_params: { operator_ids: [] },
      config: baseConfig,
    },
  };
}

// ---- nanosToMs -------------------------------------------------------------

describe('nanosToMs', () => {
  it('converts zero', () => {
    expect(nanosToMs(0n)).toBe(0);
  });

  it('converts an exact millisecond', () => {
    expect(nanosToMs(1_000_000n)).toBe(1);
  });

  it('preserves sub-millisecond precision', () => {
    expect(nanosToMs(500_000n)).toBe(0.5);
    expect(nanosToMs(1_500_000n)).toBe(1.5);
  });

  it('converts a full second', () => {
    expect(nanosToMs(1_000_000_000n)).toBe(1000);
  });

  it('handles large epoch-scale values', () => {
    // 2e15 ns = 2e9 ms
    expect(nanosToMs(2_000_000_000_000_000n)).toBe(2_000_000_000);
  });

  it('accepts a number zero', () => {
    expect(nanosToMs(0)).toBe(0);
  });

  it('accepts a number for an exact millisecond', () => {
    expect(nanosToMs(1_000_000)).toBe(1);
  });

  it('accepts a number and preserves sub-millisecond precision', () => {
    expect(nanosToMs(500_000)).toBe(0.5);
    expect(nanosToMs(1_500_000)).toBe(1.5);
  });

  it('accepts a number for a full second', () => {
    expect(nanosToMs(1_000_000_000)).toBe(1000);
  });
});

// ---- getLongEntitiesThreshold ----------------------------------------------

describe('getLongEntitiesThreshold', () => {
  it('uses the middle density threshold by default', () => {
    expect(getLongEntitiesThreshold(200, 200)).toBe(1);
  });

  it.each([
    [1, 100],
    [2, 10],
    [3, 1],
    [4, 0.1],
    [5, 0.01],
  ] as const)('maps density %s to its bin multiplier', (density, expected) => {
    expect(getLongEntitiesThreshold(200, 200, density)).toBe(expected);
  });

  it('scales linearly with the visible window', () => {
    expect(getLongEntitiesThreshold(100, 200)).toBe(0.5);
    expect(getLongEntitiesThreshold(400, 200)).toBe(2);
  });

  it('uses the returned bin count', () => {
    expect(getLongEntitiesThreshold(200, 400)).toBe(0.5);
  });

  it('returns 0 for a zero-second window', () => {
    expect(getLongEntitiesThreshold(0, 200)).toBe(0);
  });
});

// ---- getTimelineXAxisIntervalMs --------------------------------------------

describe('getTimelineXAxisIntervalMs', () => {
  it.each([
    [700, 100],
    [1_400, 200],
    [3_500, 500],
    [7_000, 1_000],
    [7 * 60_000, 60_000],
    [7 * 3_600_000, 3_600_000],
    [7 * 86_400_000, 86_400_000],
  ])('picks the right nice interval for span %i ms', (span, expected) => {
    expect(getTimelineXAxisIntervalMs(span)).toBe(expected);
  });

  it('falls back to the raw step when the span is smaller than any nice interval', () => {
    // 10ms span, 2 target splits → maxAllowedStep = 10 / 1 = 10; even 100ms is too coarse
    expect(getTimelineXAxisIntervalMs(10, 2)).toBe(10);
  });

  it('respects a custom targetSplits that allows a coarser interval', () => {
    // 7s span, 2 splits → maxAllowedStep = 7000 / 1 = 7000 → picks 5-second interval
    expect(getTimelineXAxisIntervalMs(7_000, 2)).toBe(5_000);
  });

  it('treats targetSplits < 2 as 2', () => {
    // Same result as targetSplits = 2
    expect(getTimelineXAxisIntervalMs(7_000, 1)).toBe(getTimelineXAxisIntervalMs(7_000, 2));
  });
});

// ---- dimSeries -------------------------------------------------------------

describe('dimSeries', () => {
  it('returns an empty object for an empty series', () => {
    expect(dimSeries({})).toEqual({});
  });

  it('sets isDimmed on every entry', () => {
    const input: TimelineSeries = {
      run: makeEntry('#f00'),
      idle: makeEntry('#0f0'),
    };
    const result = dimSeries(input);
    expect(result.run?.isDimmed).toBe(true);
    expect(result.idle?.isDimmed).toBe(true);
  });

  it('overrides an existing false isDimmed', () => {
    const input: TimelineSeries = { run: makeEntry('#f00', { isDimmed: false }) };
    expect(dimSeries(input).run?.isDimmed).toBe(true);
  });

  it('does not mutate the input entries', () => {
    const entry = makeEntry('#f00');
    dimSeries({ run: entry });
    expect(entry.isDimmed).toBeUndefined();
  });

  it('preserves other entry fields unchanged', () => {
    const input: TimelineSeries = { run: makeEntry('#f00') };
    const result = dimSeries(input);
    expect(result.run?.color).toBe('#f00');
    expect(result.run?.binDuration).toBe(1000);
  });
});

// ---- mergeOverlaySeries ----------------------------------------------------

describe('mergeOverlaySeries', () => {
  it('dims all base entries in the result', () => {
    const base: TimelineSeries = { run: makeEntry('#f00') };
    const result = mergeOverlaySeries(base, {}, 'op-1');
    expect(result.run?.isDimmed).toBe(true);
  });

  it('adds overlay entries with the overlayLabel appended to the key', () => {
    const base: TimelineSeries = { run: makeEntry('#f00') };
    const overlay: TimelineSeries = { run: makeEntry('#0f0') };
    const result = mergeOverlaySeries(base, overlay, 'op-1');
    expect('run (op-1)' in result).toBe(true);
    expect(result['run (op-1)']?.isOverlay).toBe(true);
  });

  it('overlay entry inherits base entry color when the state name matches', () => {
    const base: TimelineSeries = { run: makeEntry('#f00') };
    const overlay: TimelineSeries = { run: makeEntry('#0f0') };
    const result = mergeOverlaySeries(base, overlay, 'op-1');
    expect(result['run (op-1)']?.color).toBe('#f00');
  });

  it('overlay entry keeps its own color when no matching base entry exists', () => {
    const base: TimelineSeries = { run: makeEntry('#f00') };
    const overlay: TimelineSeries = { wait: makeEntry('#00f') };
    const result = mergeOverlaySeries(base, overlay, 'op-1');
    expect(result['wait (op-1)']?.color).toBe('#00f');
  });

  it('overlay entries are not dimmed', () => {
    const base: TimelineSeries = { run: makeEntry('#f00') };
    const overlay: TimelineSeries = { run: makeEntry('#0f0') };
    const result = mergeOverlaySeries(base, overlay, 'op-1');
    expect(result['run (op-1)']?.isDimmed).toBeUndefined();
  });

  it('does not mutate the base series', () => {
    const entry = makeEntry('#f00');
    const base: TimelineSeries = { run: entry };
    mergeOverlaySeries(base, {}, 'op-1');
    expect(entry.isDimmed).toBeUndefined();
  });
});

describe('computeVisibleMaxValue', () => {
  it('uses operator overlays instead of the dimmed full-data series', () => {
    const series: TimelineSeries = {
      base: makeEntry('#f00', { values: [100, 100], binDuration: 1, isDimmed: true }),
      running: makeEntry('#0f0', { values: [2, 3], binDuration: 1, isOverlay: true }),
      waiting: makeEntry('#00f', { values: [4, 1], binDuration: 1, isOverlay: true }),
    };

    expect(computeVisibleMaxValue(series, [0, 1000], 0, 2000)).toBe(6);
  });

  it('uses regular series when no overlay is active', () => {
    const series: TimelineSeries = {
      running: makeEntry('#0f0', { values: [2, 3], binDuration: 1 }),
      waiting: makeEntry('#00f', { values: [4, 1], binDuration: 1 }),
    };

    expect(computeVisibleMaxValue(series, [0, 1000], 0, 2000)).toBe(6);
  });
});

// ---- setOperatorOnEntry ----------------------------------------------------

describe('setOperatorOnEntry', () => {
  it('sets operator_ids on a Resource entry', () => {
    const entry = makeResourceEntry();
    const updated = setOperatorOnEntry(entry, ['op-42', 'op-43']);
    const opIds = 'Resource' in updated ? updated.Resource.application.operator_ids : [];
    expect(opIds).toEqual(['op-42', 'op-43']);
  });

  it('sets operator_ids on a ResourceGroup entry', () => {
    const entry = makeGroupEntry();
    const updated = setOperatorOnEntry(entry, ['op-42', 'op-43']);
    const opIds = 'ResourceGroup' in updated ? updated.ResourceGroup.app_params.operator_ids : [];
    expect(opIds).toEqual(['op-42', 'op-43']);
  });

  it('does not mutate the original Resource entry', () => {
    const entry = makeResourceEntry();
    setOperatorOnEntry(entry, ['op-42']);
    const origOpIds = 'Resource' in entry ? entry.Resource.application.operator_ids : ['mutated'];
    expect(origOpIds).toEqual([]);
  });

  it('does not mutate the original ResourceGroup entry', () => {
    const entry = makeGroupEntry();
    setOperatorOnEntry(entry, ['op-42']);
    const origOpIds =
      'ResourceGroup' in entry ? entry.ResourceGroup.app_params.operator_ids : ['mutated'];
    expect(origOpIds).toEqual([]);
  });

  it('preserves other fields on a Resource entry', () => {
    const entry = makeResourceEntry();
    const updated = setOperatorOnEntry(entry, ['op-42']);
    const id = 'Resource' in updated ? updated.Resource.resource_id : null;
    expect(id).toBe('r1');
  });

  it('preserves other fields on a ResourceGroup entry', () => {
    const entry = makeGroupEntry();
    const updated = setOperatorOnEntry(entry, ['op-42']);
    const typeName = 'ResourceGroup' in updated ? updated.ResourceGroup.resource_type_name : null;
    expect(typeName).toBe('disk');
  });
});

// ---- setOperatorOnEntries --------------------------------------------------

describe('setOperatorOnEntries', () => {
  it('applies the operator to all entries in the record', () => {
    const entries = { r1: makeResourceEntry(), g1: makeGroupEntry() };
    const updated = setOperatorOnEntries(entries, ['op-98', 'op-99']);
    const r1OpIds = 'Resource' in updated.r1 ? updated.r1.Resource.application.operator_ids : [];
    const g1OpIds =
      'ResourceGroup' in updated.g1 ? updated.g1.ResourceGroup.app_params.operator_ids : [];
    expect(r1OpIds).toEqual(['op-98', 'op-99']);
    expect(g1OpIds).toEqual(['op-98', 'op-99']);
  });

  it('returns a new record without mutating the input', () => {
    const entries = { r1: makeResourceEntry() };
    setOperatorOnEntries(entries, ['op-99']);
    const origOpIds =
      'Resource' in entries.r1 ? entries.r1.Resource.application.operator_ids : ['mutated'];
    expect(origOpIds).toEqual([]);
  });

  it('returns an empty record for an empty input', () => {
    expect(setOperatorOnEntries({}, ['op-1'])).toEqual({});
  });
});

// ---- findItemById ----------------------------------------------------------

describe('findItemById', () => {
  const leaf = makeTreeItem('leaf');
  const sibling = makeTreeItem('sibling');
  const parent = makeTreeItem('parent', [leaf, sibling]);
  const root = makeTreeItem('root', [parent]);

  it('returns the root when the id matches the root', () => {
    expect(findItemById(root, 'root')).toBe(root);
  });

  it('finds a direct child', () => {
    expect(findItemById(root, 'parent')).toBe(parent);
  });

  it('finds a deeply nested item', () => {
    expect(findItemById(root, 'leaf')).toBe(leaf);
  });

  it('finds a sibling at the same depth', () => {
    expect(findItemById(root, 'sibling')).toBe(sibling);
  });

  it('returns undefined when the id does not exist in the tree', () => {
    expect(findItemById(root, 'missing')).toBeUndefined();
  });

  it('works on a leaf node with no children', () => {
    expect(findItemById(leaf, 'leaf')).toBe(leaf);
    expect(findItemById(leaf, 'other')).toBeUndefined();
  });
});

// ---- buildTimelineMarks attributes ------------------------------------------

import { buildTimelineMarks } from './timeline.utils';
import type { DynamicValue, FiniteStateMachine } from '@quent/utils';

const taggedValue = (v: object) => v as unknown as DynamicValue;

const THREAD_ID = 'aaaaaaaa-0000-0000-0000-000000000001';

const taskFsm: FiniteStateMachine = {
  id: 'bbbbbbbb-0000-0000-0000-000000000001',
  type_name: 'task',
  instance_name: 'task-0',
  transitions: [
    {
      name: 'queueing',
      usages: [],
      timestamp: 1.0,
      attributes: [{ key: 'operator_id', value: taggedValue({ String: 'op-1' }) }],
      derived_attributes: [],
    },
    {
      name: 'computing',
      usages: [{ resource: THREAD_ID, capacities: [] }],
      timestamp: 1.25,
      attributes: [{ key: 'input_bytes', value: taggedValue({ U64: 1_500_000_000 }) }],
      derived_attributes: [{ key: 'bytes_per_sec', value: taggedValue({ F64: 2_000_000_000 }) }],
    },
    { name: 'exit', usages: [], timestamp: 2.0, attributes: [], derived_attributes: [] },
  ],
};

describe('buildTimelineMarks attributes', () => {
  it('copies recorded and derived attributes onto marks', () => {
    const marks = buildTimelineMarks([taskFsm], 'light', new Set([THREAD_ID]));
    expect(marks).toBeDefined();
    // Only the computing transition has a usage on the filtered resource.
    expect(marks).toHaveLength(1);
    const mark = marks![0]!;
    expect(mark.stateName).toBe('computing');
    expect(mark.attributes).toEqual([{ key: 'input_bytes', value: { U64: 1_500_000_000 } }]);
    expect(mark.derivedAttributes).toEqual([
      { key: 'bytes_per_sec', value: { F64: 2_000_000_000 } },
    ]);
    // 1.25s → 2.0s in chart ms.
    expect(mark.xStart).toBe(1250);
    expect(mark.xEnd).toBe(2000);
  });

  it('omits attribute keys for attribute-less transitions', () => {
    const marks = buildTimelineMarks([taskFsm], 'light', null);
    expect(marks).toHaveLength(2);
    const queueing = marks!.find(m => m.stateName === 'queueing')!;
    expect(queueing.derivedAttributes).toBeUndefined();
    const computing = marks!.find(m => m.stateName === 'computing')!;
    expect(computing.attributes).toHaveLength(1);
  });

  it('tolerates responses from servers predating attributes', () => {
    const legacyFsm = {
      ...taskFsm,
      transitions: taskFsm.transitions.map(t => {
        const { attributes: _attributes, derived_attributes: _derived, ...rest } = t;
        return rest;
      }),
    } as unknown as FiniteStateMachine;
    const marks = buildTimelineMarks([legacyFsm], 'light', new Set([THREAD_ID]));
    expect(marks).toHaveLength(1);
    expect(marks![0]!.attributes).toBeUndefined();
    expect(marks![0]!.derivedAttributes).toBeUndefined();
  });
});
