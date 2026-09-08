// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render } from '@testing-library/react';
import type { EChartsInstance } from 'echarts-for-react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { usePlayheadLinePixel } from '../lib/usePlayheadLinePixel';
import { PlayheadLine } from './PlayheadLine';

const mocks = vi.hoisted(() => ({
  isPlaying: false,
  pixelX: 24 as number | null,
}));

vi.mock('@quent/hooks', () => ({
  useDataFlowIsPlaying: () => mocks.isPlaying,
}));

vi.mock('../lib/usePlayheadLinePixel', () => ({
  usePlayheadLinePixel: vi.fn(() => mocks.pixelX),
}));

describe('PlayheadLine', () => {
  afterEach(() => {
    mocks.isPlaying = false;
    mocks.pixelX = 24;
    vi.mocked(usePlayheadLinePixel).mockClear();
  });

  it('renders nothing when the playhead has no pixel position', () => {
    mocks.pixelX = null;
    const { container } = render(<PlayheadLine instance={null} />);
    expect(container).toBeEmptyDOMElement();
  });

  it('positions the overlay at the computed pixel, including zero', () => {
    const { container, rerender } = render(<PlayheadLine instance={null} />);
    expect(container.firstElementChild).toHaveStyle({ left: '24px' });

    mocks.pixelX = 0;
    rerender(<PlayheadLine instance={null} />);
    expect(container.firstElementChild).toHaveStyle({ left: '0px' });
  });

  it('only transitions position during data-flow playback', () => {
    const { container, rerender } = render(<PlayheadLine instance={null} />);
    expect(container.firstElementChild).not.toHaveClass('transition-[left]');

    mocks.isPlaying = true;
    rerender(<PlayheadLine instance={null} />);

    expect(container.firstElementChild).toHaveClass(
      'transition-[left]',
      'duration-100',
      'ease-linear',
      'motion-reduce:transition-none'
    );

    mocks.isPlaying = false;
    rerender(<PlayheadLine instance={null} />);
    expect(container.firstElementChild).not.toHaveClass('transition-[left]');
  });

  it('forwards the chart instance and x-axis index', () => {
    const instance = {} as EChartsInstance;
    const { rerender } = render(<PlayheadLine instance={instance} />);
    expect(usePlayheadLinePixel).toHaveBeenCalledWith(instance, 0);

    rerender(<PlayheadLine instance={instance} xAxisIndex={1} />);
    expect(usePlayheadLinePixel).toHaveBeenCalledWith(instance, 1);
  });
});
