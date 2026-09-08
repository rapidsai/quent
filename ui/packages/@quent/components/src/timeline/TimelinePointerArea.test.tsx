// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { fireEvent, render, screen } from '@testing-library/react';
import { Provider } from 'jotai';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { TimelinePointerArea } from './TimelinePointerArea';

const chartRect = {
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

describe('TimelinePointerArea', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('positions and clears the shared line from DOM pointer movement', () => {
    vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
      callback(0);
      return 1;
    });
    render(
      <Provider>
        <TimelinePointerArea data-testid="chart">
          <span />
        </TimelinePointerArea>
      </Provider>
    );
    const chart = screen.getByTestId('chart');
    vi.spyOn(chart, 'getBoundingClientRect').mockReturnValue(chartRect);

    fireEvent.pointerMove(chart, { clientX: 50, clientY: 20 });

    expect(chart.lastElementChild).toHaveStyle({ left: 'calc(50% + -5px)' });
    fireEvent.pointerLeave(chart);
    expect(chart.children).toHaveLength(1);
  });

  it('maps controller pointers through the selected data-zoom range', () => {
    const range = { start: 0.25, end: 0.75 };
    render(
      <Provider>
        <TimelinePointerArea data-testid="controller" range={range}>
          <span />
        </TimelinePointerArea>
        <TimelinePointerArea data-testid="detail">
          <span />
        </TimelinePointerArea>
      </Provider>
    );
    const controller = screen.getByTestId('controller');
    const detail = screen.getByTestId('detail');
    vi.spyOn(controller, 'getBoundingClientRect').mockReturnValue(chartRect);

    fireEvent.pointerMove(controller, { clientX: 25, clientY: 20 });

    expect(controller.lastElementChild).toHaveStyle({ left: 'calc(25% + -2.5px)' });
    expect(detail.lastElementChild).toHaveStyle({ left: 'calc(0% + 0px)' });
  });
});
