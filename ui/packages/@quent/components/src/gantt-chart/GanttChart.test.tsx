// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { act, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { GanttChart } from './GanttChart';

const mocks = vi.hoisted(() => ({
  clear: vi.fn(),
  publish: vi.fn(),
  handleChartReady: vi.fn(),
  instanceRef: { current: null },
  props: null as Record<string, unknown> | null,
}));

vi.mock('@quent/hooks', () => ({
  useTimelinePointerPublisher: () => ({ clear: mocks.clear, publish: mocks.publish }),
  useTimelinePointerRatio: () => 0.5,
}));
vi.mock('../lib/useChartConnect', () => ({
  useChartConnect: () => ({
    handleChartReady: mocks.handleChartReady,
    instanceRef: mocks.instanceRef,
  }),
}));
vi.mock('../timeline/PlayheadLine', () => ({ PlayheadLine: () => <div>playhead slot</div> }));
vi.mock('../ui/charts/GanttChartCore', () => ({
  GanttChartCore: (props: Record<string, unknown>) => {
    mocks.props = props;
    return <div>gantt core</div>;
  },
}));

describe('GanttChart adapter', () => {
  it('maps Quent hooks and playhead into controlled core props', () => {
    render(
      <GanttChart
        data={[]}
        durationSeconds={10}
        height={45}
        maxHeight={45}
        rowHeight={12}
        isDark={false}
        seriesName="ranges"
        renderItem={() => null}
        emptyState={<span>empty</span>}
      />
    );

    expect(screen.getByText('gantt core')).toBeVisible();
    expect(mocks.props).toMatchObject({
      pointerRatio: 0.5,
      onChartReady: mocks.handleChartReady,
    });
    act(() => (mocks.props?.onPointerRatioChange as (ratio: number | null) => void)(0.4));
    expect(mocks.publish).toHaveBeenCalledWith(0.4);
    expect((mocks.props?.renderOverlay as (instance: null) => unknown)(null)).not.toBeNull();
  });
});
