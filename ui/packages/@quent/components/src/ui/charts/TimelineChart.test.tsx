// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { TimelineChart } from './TimelineChart';

vi.mock('./echartsReactCore', () => ({ EChartsReactCore: () => <div data-testid="echarts" /> }));
vi.mock('./echarts', () => ({ echarts: {} }));
vi.mock('./timelineTheme', () => ({
  DIMMED_OPACITY: 0.25,
  MARK_AREA_BORDER_OPACITY: 0.75,
  MARK_AREA_FILL_OPACITY: 0.12,
  MARK_LABEL_TEXT_COLOR: '#fff',
  ROLLUP_TIMELINE_COLOR_DARK: '#777',
  ROLLUP_TIMELINE_COLOR_LIGHT: '#aaa',
  TIMELINE_MONO_FONT: 'monospace',
  useTimelineEchartsTheme: () => ({
    themeName: 'test-theme',
    textColor: '#111',
    labelBackgroundColor: '#fff',
  }),
}));

describe('TimelineChart', () => {
  it('renders controlled timeline content without external providers', () => {
    render(
      <TimelineChart
        durationSeconds={2}
        series={{
          running: {
            binDuration: 1,
            formatter: value => `${value} units`,
            values: [5, 3],
            color: '#0f0',
          },
        }}
        timestamps={[0, 1000]}
        isDark={false}
        yAxisLabel="Capacity"
        visibleStartMs={0}
        visibleEndMs={2000}
        minZoomWindowSeconds={0.001}
        pointerRatio={null}
        onPointerRatioChange={vi.fn()}
      />
    );

    expect(screen.getByTestId('echarts')).toBeVisible();
    expect(screen.getByText('5 units')).toBeVisible();
    expect(screen.getByText('Capacity')).toBeVisible();
  });
});
