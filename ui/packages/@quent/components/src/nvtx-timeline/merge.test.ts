// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import type { NvtxLane, NvtxMarkItem, NvtxRangeItem } from '@quent/utils';
import {
  mergeNvtxGanttData,
  NVTX_BAR_MERGE_MIN_COUNT,
  nvtxItemsAtTimestamp,
  nvtxLanesToGanttData,
  nvtxMergedBarCountLabel,
  nvtxTooltipModel,
  type NvtxGanttDatum,
} from './utils';

const budget = { visibleStartMs: 0, visibleEndMs: 100, plotWidthPx: 100 };

function touchingBars(count: number): NvtxGanttDatum[] {
  return Array.from({ length: count }, (_, index) => ({
    value: [index, index + 1, 0],
  }));
}

describe('NVTX Gantt condensation', () => {
  it('leaves runs below the minimum count separate', () => {
    const bars = touchingBars(NVTX_BAR_MERGE_MIN_COUNT - 1);

    expect(mergeNvtxGanttData(bars, budget)).toEqual(bars);
  });

  it('condenses runs at the minimum count', () => {
    const merged = mergeNvtxGanttData(touchingBars(NVTX_BAR_MERGE_MIN_COUNT), budget);

    expect(merged).toHaveLength(1);
    expect(merged[0]).toMatchObject({
      value: [0, NVTX_BAR_MERGE_MIN_COUNT, 0],
      mergedCount: NVTX_BAR_MERGE_MIN_COUNT,
    });
  });

  it('labels a consolidated block with its range count', () => {
    const [label] = nvtxMergedBarCountLabel(
      { x: 10, y: 0, width: 80, height: 14 },
      '#111',
      NVTX_BAR_MERGE_MIN_COUNT,
      'range'
    );

    expect(label?.style).toMatchObject({
      text: `(${NVTX_BAR_MERGE_MIN_COUNT} ranges)`,
      opacity: 0.6,
    });
  });

  it('omits a count that cannot fit without truncation', () => {
    expect(
      nvtxMergedBarCountLabel({ x: 10, y: 0, width: 4, height: 14 }, '#111', 12, 'range')
    ).toEqual([]);
  });

  it('labels consolidated marks as marks', () => {
    const [label] = nvtxMergedBarCountLabel(
      { x: 10, y: 0, width: 80, height: 14 },
      '#111',
      NVTX_BAR_MERGE_MIN_COUNT,
      'mark'
    );

    expect(label?.style).toMatchObject({ text: `(${NVTX_BAR_MERGE_MIN_COUNT} marks)` });
  });
});

function rangeDatum(message: string, depth: number, startMs = 0): NvtxGanttDatum {
  const range: NvtxRangeItem = {
    message,
    domain_id: 'domain-1',
    domain_name: 'Domain 1',
    category_id: null,
    category_name: null,
    color: '#76b900ff',
    kind: 'push_pop',
    thread_id: 42,
    thread_name: 'worker 42',
    observed_start: startMs / 1_000,
    observed_end: (startMs + 1) / 1_000,
    display_start: startMs / 1_000,
    display_end: (startMs + 1) / 1_000,
    observed_duration: 0.001,
    incomplete: false,
  };
  return { value: [startMs, startMs + 1, depth], range };
}

function markDatum(message: string, timestampMs = 0): NvtxGanttDatum {
  const mark: NvtxMarkItem = {
    message,
    domain_id: 'domain-1',
    domain_name: 'Domain 1',
    category_id: null,
    category_name: null,
    color: '#76b900ff',
    timestamp: timestampMs / 1_000,
  };
  return { value: [timestampMs, timestampMs, 0], mark };
}

function threadLane(depth: number, ranges: NvtxRangeItem[] = []): NvtxLane {
  return {
    id: `thread-42-depth-${depth}`,
    label: `worker 42 depth ${depth}`,
    identity: { kind: 'thread', thread_id: 42, depth },
    ranges,
    marks: [],
  };
}

describe('NVTX Gantt lanes', () => {
  it('compacts populated lanes instead of preserving empty depth gaps', () => {
    const data = nvtxLanesToGanttData([
      threadLane(0, [rangeDatum('outer', 0).range!]),
      threadLane(1),
      threadLane(2, [rangeDatum('inner', 2).range!]),
    ]);

    expect(data.map(datum => [datum.range?.message, datum.value[2]])).toEqual([
      ['outer', 0],
      ['inner', 1],
    ]);
  });

  it('returns no chart lanes when the thread row is empty', () => {
    expect(nvtxLanesToGanttData([threadLane(0), threadLane(1)])).toEqual([]);
  });
});

describe('NVTX Gantt tooltip', () => {
  it('includes the thread and orders ranges by chart depth', () => {
    const tooltip = nvtxTooltipModel([
      rangeDatum('inner', 2),
      rangeDatum('outer', 0),
      rangeDatum('middle', 1),
    ]);

    expect(tooltip.marks.map(mark => mark.label)).toEqual(['outer', 'middle', 'inner']);
    expect(tooltip.marks.map(mark => mark.stateName)).toEqual(['', '', '']);
    expect(tooltip.marks[0]?.attributes).toContainEqual({
      key: 'thread',
      value: 'worker 42',
    });
    expect(tooltip.marks[0]?.attributes).toContainEqual({
      key: 'thread ID',
      value: '42',
    });
  });

  it('hit-tests instant marks using the rendered pixel width', () => {
    const datum = markDatum('instant', 10);

    expect(nvtxItemsAtTimestamp([datum], 11.9, 2)).toEqual([datum]);
    expect(nvtxItemsAtTimestamp([datum], 12, 2)).toEqual([]);
  });

  it('names hidden items for the active NVTX kind', () => {
    expect(nvtxTooltipModel([rangeDatum('range', 0)]).itemNoun).toEqual({
      singular: 'range',
      plural: 'ranges',
    });
    expect(nvtxTooltipModel([markDatum('mark')]).itemNoun).toEqual({
      singular: 'mark',
      plural: 'marks',
    });
  });

  it('aggregates consolidated counts by range type', () => {
    const data = [
      ...Array.from({ length: 3 }, (_, index) => rangeDatum('type A', 0, index)),
      ...Array.from({ length: 5 }, (_, index) => rangeDatum('type B', 0, index + 3)),
    ];
    const tooltip = nvtxTooltipModel(mergeNvtxGanttData(data, budget));

    expect(tooltip.summary).toBe('8 ranges');
    expect(tooltip.itemNoun).toEqual({ singular: 'range', plural: 'ranges' });
    expect(tooltip.marks).toEqual([
      { label: 'type A', stateName: '3 ranges', color: '#76b900', compact: true },
      { label: 'type B', stateName: '5 ranges', color: '#76b900', compact: true },
    ]);
  });

  it('aggregates consolidated counts by mark type', () => {
    const data = [
      ...Array.from({ length: 3 }, (_, index) => markDatum('type A', index)),
      ...Array.from({ length: 5 }, (_, index) => markDatum('type B', index + 3)),
    ];
    const tooltip = nvtxTooltipModel(mergeNvtxGanttData(data, budget));

    expect(tooltip.summary).toBe('8 marks');
    expect(tooltip.itemNoun).toEqual({ singular: 'mark', plural: 'marks' });
    expect(tooltip.marks).toEqual([
      { label: 'type A', stateName: '3 marks', color: '#76b900', compact: true },
      { label: 'type B', stateName: '5 marks', color: '#76b900', compact: true },
    ]);
  });
});
