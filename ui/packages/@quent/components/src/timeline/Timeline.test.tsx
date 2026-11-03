// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { act, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { Timeline } from './Timeline';

const mocks = vi.hoisted(() => ({
  clear: vi.fn(),
  publish: vi.fn(),
  handleChartReady: vi.fn(),
  props: null as Record<string, unknown> | null,
}));

vi.mock('@quent/hooks', () => ({
  useTimelinePointerPublisher: () => ({ clear: mocks.clear, publish: mocks.publish }),
  useTimelinePointerRatio: () => 0.25,
  useZoomRange: () => ({ start: 2, end: 5 }),
}));
vi.mock('../lib/useChartConnect', () => ({
  useChartConnect: () => ({ handleChartReady: mocks.handleChartReady }),
}));
vi.mock('../ui/charts/TimelineChart', () => ({
  TimelineChart: (props: Record<string, unknown>) => {
    mocks.props = props;
    return <div>timeline core</div>;
  },
}));

describe('Timeline adapter', () => {
  it('maps Quent hooks into controlled core props', () => {
    render(
      <Timeline
        durationSeconds={10}
        series={{}}
        timestamps={[]}
        isDark={false}
        marks={[
          {
            label: 'Wait',
            stateName: 'Blocked',
            operatorName: 'Join',
            color: '#f00',
            xStart: 0,
            xEnd: 1,
          },
        ]}
      />
    );

    expect(screen.getByText('timeline core')).toBeVisible();
    expect(mocks.props).toMatchObject({
      visibleStartMs: 2000,
      visibleEndMs: 5000,
      pointerRatio: 0.25,
      onChartReady: mocks.handleChartReady,
      marks: [
        {
          labelLines: ['Wait', 'Blocked', 'Join'],
          color: '#f00',
          xStart: 0,
          xEnd: 1,
        },
      ],
    });
    act(() => (mocks.props?.onPointerRatioChange as (ratio: number | null) => void)(0.4));
    expect(mocks.publish).toHaveBeenCalledWith(0.4);
    act(() => (mocks.props?.onPointerRatioChange as (ratio: number | null) => void)(null));
    expect(mocks.clear).toHaveBeenCalledOnce();
  });
});
