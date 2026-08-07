// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import type { NvtxLane } from '@quent/utils';
import { NvtxLaneChart } from './NvtxLaneChart';
import { formatNvtxDuration, nvtxRelativeSecondsToMs } from './NvtxLaneChart.utils';

vi.mock('@quent/hooks', () => ({ useZoomRange: () => ({ start: 0, end: 1 }) }));
vi.mock('../gantt-chart/GanttChart', () => ({
  GanttChart: () => <div data-testid="gantt" />,
}));

const lane: NvtxLane = {
  id: 'lane',
  label: 'thread 7',
  identity: { kind: 'thread', thread_id: 7, depth: 0 },
  marks: [],
  ranges: [
    {
      message: '<script>work</script>',
      domain_id: '1',
      domain_name: 'Runtime',
      category_id: null,
      category_name: null,
      color: '#2563eb',
      kind: 'push_pop',
      thread_id: 7,
      thread_name: 'worker',
      observed_start: 0.00000001,
      observed_end: null,
      display_start: 0.00000001,
      display_end: 0.10000001,
      observed_duration: null,
      incomplete: true,
    },
  ],
};

describe('NvtxLaneChart', () => {
  it('converts and formats relative-second contract values', () => {
    expect(nvtxRelativeSecondsToMs(0.00000001)).toBe(0.00001);
    expect(formatNvtxDuration(0.00125)).toBe('1.25ms');
  });

  it('keeps the keyboard tooltip and exposes the message without injecting HTML', () => {
    render(<NvtxLaneChart lanes={[lane]} durationSeconds={1} isDark={false} />);
    const target = screen.getByRole('button', { name: /script.*incomplete/i });
    expect(target).toHaveAttribute('aria-label', '<script>work</script>, incomplete');
    fireEvent.focus(target);
    const tooltip = screen.getByRole('tooltip');
    expect(tooltip).toHaveTextContent('<script>work</script>');
    expect(tooltip).toHaveTextContent('incomplete');
    expect(tooltip).toHaveTextContent('open at trace boundary');
    expect(tooltip).toHaveTextContent('worker');
    expect(document.querySelector('script')).toBeNull();
  });
});
