// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { SegmentedBar } from './SegmentedBar';

const segments = [
  {
    id: 'running',
    value: 2,
    color: '#76b900',
    label: '2',
    tooltip: <span>Running: 2</span>,
    ariaLabel: 'running: 20%',
  },
];

describe('SegmentedBar', () => {
  it('configures height, fill scaling, labels, and tooltips', () => {
    const { container, rerender } = render(
      <SegmentedBar
        segments={segments}
        fillValue={2}
        maxValue={10}
        height={8}
        showLabels={false}
        showTooltips={false}
        labelTestId="segment-label"
      />
    );

    const track = container.firstElementChild as HTMLElement;
    const fill = track.firstElementChild as HTMLElement;
    const segment = screen.getByRole('img', { name: 'running: 20%' });
    expect(track.style.height).toBe('8px');
    expect(fill.style.width).toBe('20%');
    expect(screen.queryByTestId('segment-label')).not.toBeInTheDocument();

    fireEvent.mouseEnter(segment, { clientX: 10, clientY: 20 });
    expect(screen.queryByText('Running: 2')).not.toBeInTheDocument();

    rerender(
      <SegmentedBar segments={segments} showLabels showTooltips labelTestId="segment-label" />
    );

    fireEvent.mouseEnter(screen.getByRole('img', { name: 'running: 20%' }), {
      clientX: 10,
      clientY: 20,
    });
    expect(screen.getByTestId('segment-label')).toHaveTextContent('2');
    expect(screen.getByTestId('segment-label')).toHaveClass('font-mono');
    expect(screen.getByText('Running: 2')).toBeInTheDocument();
  });
});
