// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ChartPointerArea } from './ChartPointerArea';

const CHART_RECT = {
  x: 0,
  y: 0,
  top: 0,
  right: 110,
  bottom: 45,
  left: 0,
  width: 110,
  height: 45,
  toJSON: () => ({}),
};

describe('ChartPointerArea', () => {
  it('reports and clears controlled pointer ratios without a provider', () => {
    const onPointerRatioChange = vi.fn();
    const { rerender } = render(
      <ChartPointerArea
        data-testid="chart"
        left={0}
        right={10}
        pointerRatio={null}
        onPointerRatioChange={onPointerRatioChange}
      >
        <span>content</span>
      </ChartPointerArea>
    );
    const chart = screen.getByTestId('chart');
    vi.spyOn(chart, 'getBoundingClientRect').mockReturnValue(CHART_RECT);

    fireEvent.pointerMove(chart, { clientX: 50, clientY: 20 });
    expect(onPointerRatioChange).toHaveBeenLastCalledWith(0.5);

    rerender(
      <ChartPointerArea
        data-testid="chart"
        left={0}
        right={10}
        pointerRatio={0.5}
        onPointerRatioChange={onPointerRatioChange}
      >
        <span>content</span>
      </ChartPointerArea>
    );
    expect(chart.querySelector('[aria-hidden="true"]')).not.toBeNull();

    fireEvent.pointerLeave(chart);
    expect(onPointerRatioChange).toHaveBeenLastCalledWith(null);
  });
});
