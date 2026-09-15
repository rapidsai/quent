// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { NvtxLane, NvtxRangeItem } from '@quent/utils';
import { NvtxGantt } from './NvtxGantt';

const mocks = vi.hoisted(() => ({
  ganttChart: vi.fn(),
}));

vi.mock('@quent/hooks', () => ({
  useDebouncedZoomRange: () => ({ start: 0, end: 1 }),
}));

vi.mock('../timeline/timelineEchartsTheme', () => ({
  MARK_AREA_BORDER_OPACITY: 0.8,
  MARK_AREA_FILL_OPACITY: 0.2,
  useTimelineEchartsTheme: () => ({ textColor: '#000000' }),
}));

vi.mock('../gantt-chart/GanttChart', () => ({
  GanttChart: (props: Record<string, unknown>) => {
    mocks.ganttChart(props);
    return <div />;
  },
}));

const mergedOverride = vi.hoisted(() => ({
  data: null as ReturnType<typeof import('./utils').mergeNvtxGanttData> | null,
}));

vi.mock('./utils', async importOriginal => {
  const actual = await importOriginal<typeof import('./utils')>();
  return {
    ...actual,
    mergeNvtxGanttData: (
      ...args: Parameters<typeof actual.mergeNvtxGanttData>
    ): ReturnType<typeof actual.mergeNvtxGanttData> =>
      mergedOverride.data ?? actual.mergeNvtxGanttData(...args),
  };
});

function range(overrides: Partial<NvtxRangeItem> = {}): NvtxRangeItem {
  return {
    message: 'work',
    span_id: 1,
    parent_span_id: null,
    domain_id: 'domain-1',
    domain_name: 'Domain 1',
    category_id: null,
    category_name: null,
    color: '#76b900ff',
    kind: 'push_pop',
    thread_id: 42,
    thread_name: 'worker 42',
    observed_start: 0,
    observed_end: 0.001,
    display_start: 0,
    display_end: 0.001,
    observed_duration: 0.001,
    payload: null,
    incomplete: false,
    ...overrides,
  };
}

function lane(ranges: NvtxRangeItem[]): NvtxLane {
  return {
    id: 'lane-1',
    label: 'thread',
    identity: { kind: 'thread', thread_id: 42, depth: 0 },
    ranges,
    marks: [],
  };
}

type MockGanttChartProps = {
  cursor?: string;
  onEvents?: { click: (params: { dataIndex: number; seriesName?: string }) => void };
};

function lastGanttChartProps(): MockGanttChartProps {
  const calls = mocks.ganttChart.mock.calls;
  return calls[calls.length - 1]?.[0] as MockGanttChartProps;
}

describe('NvtxGantt', () => {
  it('does not wire click handling when onRangeClick is absent', () => {
    render(<NvtxGantt lanes={[lane([range()])]} durationSeconds={1} isDark={false} />);

    const props = lastGanttChartProps();
    expect(props.cursor).toBeUndefined();
    expect(props.onEvents).toBeUndefined();
  });

  it('invokes onRangeClick for a single range bar, ignoring other series', () => {
    const onRangeClick = vi.fn();
    render(
      <NvtxGantt
        lanes={[lane([range()])]}
        durationSeconds={1}
        isDark={false}
        onRangeClick={onRangeClick}
      />
    );

    const props = lastGanttChartProps();
    expect(props.cursor).toBe('pointer');

    props.onEvents?.click({ dataIndex: 0, seriesName: 'nvtx-range' });
    expect(onRangeClick).toHaveBeenCalledWith(expect.objectContaining({ span_id: 1 }));

    onRangeClick.mockClear();
    props.onEvents?.click({ dataIndex: 0, seriesName: 'other-series' });
    expect(onRangeClick).not.toHaveBeenCalled();
  });

  it('ignores clicks on a merged block, which has no single span to open', () => {
    mergedOverride.data = [{ value: [0, 1, 0], range: range(), mergedCount: 10 }];
    const onRangeClick = vi.fn();
    render(
      <NvtxGantt
        lanes={[lane([range()])]}
        durationSeconds={1}
        isDark={false}
        onRangeClick={onRangeClick}
      />
    );

    const props = lastGanttChartProps();
    props.onEvents?.click({ dataIndex: 0, seriesName: 'nvtx-range' });
    expect(onRangeClick).not.toHaveBeenCalled();
    mergedOverride.data = null;
  });
});
