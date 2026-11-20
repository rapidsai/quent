// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { GanttChartCore } from './GanttChartCore';

vi.mock('./echartsReactCore', () => ({ EChartsReactCore: () => <div data-testid="echarts" /> }));
vi.mock('./echarts', () => ({ echarts: {} }));
vi.mock('./timelineTheme', () => ({
  useTimelineEchartsTheme: () => ({ themeName: 'test-theme' }),
}));

describe('GanttChartCore', () => {
  it('renders controlled empty and overlay slots without external providers', () => {
    render(
      <GanttChartCore
        data={[]}
        durationSeconds={2}
        height={45}
        maxHeight={45}
        rowHeight={12}
        isDark={false}
        seriesName="ranges"
        renderItem={() => null}
        emptyState={<span>No ranges</span>}
        minZoomWindowSeconds={0.001}
        pointerRatio={null}
        onPointerRatioChange={vi.fn()}
        renderOverlay={() => <span>Overlay slot</span>}
      />
    );

    expect(screen.getByTestId('echarts')).toBeVisible();
    expect(screen.getByText('No ranges')).toBeVisible();
    expect(screen.getByText('Overlay slot')).toBeVisible();
  });
});
