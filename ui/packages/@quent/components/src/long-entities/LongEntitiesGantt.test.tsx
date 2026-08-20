// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { ReactNode } from 'react';
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { LongEntitiesGantt } from './LongEntitiesGantt';
import type { LongEntityEntry } from './types';

const mocks = vi.hoisted(() => ({
  ganttChart: vi.fn(),
}));

vi.mock('@quent/hooks', () => ({
  useZoomRange: () => ({ start: 0, end: 1 }),
}));

vi.mock('../timeline/timelineEchartsTheme', () => ({
  MARK_AREA_BORDER_OPACITY: 0.8,
  MARK_AREA_FILL_OPACITY: 0.2,
  useTimelineEchartsTheme: () => ({ textColor: '#000000' }),
}));

vi.mock('../gantt-chart/GanttChart', () => ({
  GanttChart: (props: {
    animateHeight: boolean;
    contentPaddingBottom: number;
    emptyMessage: ReactNode;
    gridSpacing: { bottom: number };
    maxHeight: number;
  }) => {
    mocks.ganttChart(props);
    return <div>{props.emptyMessage}</div>;
  },
}));

describe('LongEntitiesGantt', () => {
  it('explains the active threshold when no entities match', () => {
    render(
      <LongEntitiesGantt entries={[]} durationSeconds={1} minUsageSeconds={0.06} isDark={false} />
    );

    expect(screen.getByText('No Matching Entities')).toBeInTheDocument();
    expect(
      screen.getByText('Showing entities longer than 60.0ms. Zoom to see more.')
    ).toBeInTheDocument();
  });

  it('expands to fit all rows and collapses to the default height', () => {
    const entries: LongEntityEntry[] = [
      {
        entityId: 'entity-1',
        label: 'Entity 1',
        typeName: 'test',
        startMs: 0,
        endMs: 100,
        rowIndex: 5,
        segments: [
          {
            stateName: 'running',
            startMs: 0,
            endMs: 100,
            color: '#76b900',
          },
        ],
      },
    ];

    render(
      <LongEntitiesGantt
        entries={entries}
        durationSeconds={1}
        minUsageSeconds={0.06}
        isDark={false}
      />
    );

    expect(mocks.ganttChart).toHaveBeenLastCalledWith(
      expect.objectContaining({
        animateHeight: true,
        contentPaddingBottom: 12,
        gridSpacing: expect.objectContaining({ bottom: 14.5 }),
        maxHeight: 75,
      })
    );

    const expandButton = screen.getByRole('button', { name: 'Expand entities chart' });
    expect(expandButton).toHaveStyle({ right: '10px' });
    expect(expandButton).toHaveClass('focus-visible:ring-0', 'focus-visible:ring-offset-0');
    fireEvent.click(expandButton);
    expect(mocks.ganttChart).toHaveBeenLastCalledWith(expect.objectContaining({ maxHeight: 96 }));

    fireEvent.click(screen.getByRole('button', { name: 'Collapse entities chart' }));
    expect(mocks.ganttChart).toHaveBeenLastCalledWith(expect.objectContaining({ maxHeight: 75 }));
  });
});
