// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it, vi } from 'vitest';
import { TIMELINE_SPACING } from '../timeline/types';
import { buildGanttOption, type GanttRenderItem } from './options';

describe('buildGanttOption', () => {
  it('builds the shared elapsed-time custom-series chart', () => {
    const data = [{ value: [100, 500, 1] as [number, number, number] }];
    const renderItem = vi.fn(() => null) as GanttRenderItem;

    const option = buildGanttOption({
      data,
      durationSeconds: 12,
      yAxisCategories: [0, 1],
      seriesName: 'test-series',
      renderItem,
      minZoomSpanPct: 2,
      cursor: 'pointer',
      gridSpacing: { left: 1, right: 2, top: 3, bottom: 4 },
    });

    expect(option).toMatchObject({
      tooltip: { show: false },
      grid: { left: 1, right: 2, top: 3, bottom: 4 },
      xAxis: {
        type: 'value',
        min: 0,
        max: 12_000,
        axisPointer: { show: false },
      },
      yAxis: { type: 'category', data: [0, 1], inverse: true },
      series: [
        {
          type: 'custom',
          name: 'test-series',
          cursor: 'pointer',
          data,
          renderItem,
          encode: { x: [0, 1], y: 2 },
        },
      ],
    });
    expect((option.dataZoom as object[])[0]).toMatchObject({ type: 'slider', minSpan: 2 });
  });
  it('uses the timeline spacing defaults when grid spacing is omitted', () => {
    const option = buildGanttOption({
      data: [],
      durationSeconds: 12,
      yAxisCategories: [],
      seriesName: 'test-series',
      renderItem: vi.fn(() => null) as GanttRenderItem,
      minZoomSpanPct: 2,
    });

    expect(option.grid).toMatchObject(TIMELINE_SPACING);
  });
});
