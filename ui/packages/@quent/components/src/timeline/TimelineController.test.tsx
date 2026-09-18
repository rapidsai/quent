// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { ReactNode } from 'react';
import { render } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { TimelineController } from './TimelineController';

const mocks = vi.hoisted(() => ({
  zoomRange: { start: 0, end: 10 },
  playheadTimeS: 8 as number | null,
  playheadLineTimeMs: 8000 as number | null,
  setIsPlaying: vi.fn(),
  setPlayheadTimeS: vi.fn(),
  setPlayheadLineTimeMs: vi.fn(),
}));

vi.mock('@quent/hooks', () => ({
  useZoomRange: () => mocks.zoomRange,
  usePlayheadTimeS: () => mocks.playheadTimeS,
  usePlayheadLineTimeMs: () => mocks.playheadLineTimeMs,
  useSetDataFlowIsPlaying: () => mocks.setIsPlaying,
  useSetPlayheadTimeS: () => mocks.setPlayheadTimeS,
  useSetPlayheadLineTimeMs: () => mocks.setPlayheadLineTimeMs,
}));

vi.mock('../lib/echartsReactCore', () => ({
  EChartsReactCore: () => null,
}));

vi.mock('../lib/useChartConnect', () => ({
  useChartConnect: () => ({ handleChartReady: vi.fn() }),
}));

vi.mock('../lib/useMinZoomSpanPct', () => ({
  useMinZoomSpanPct: () => 1,
}));

vi.mock('./timelineEchartsTheme', () => ({
  useTimelineEchartsTheme: () => ({
    themeName: 'light',
    controllerGridBackgroundColor: 'transparent',
  }),
}));

vi.mock('./TimelinePointerArea', () => ({
  TimelinePointerArea: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));

vi.mock('./PlayheadLine', () => ({
  PlayheadLine: () => null,
}));

describe('TimelineController', () => {
  afterEach(() => {
    mocks.zoomRange = { start: 0, end: 10 };
    mocks.playheadTimeS = 8;
    mocks.playheadLineTimeMs = 8000;
    mocks.setIsPlaying.mockReset();
    mocks.setPlayheadTimeS.mockReset();
    mocks.setPlayheadLineTimeMs.mockReset();
  });

  it('clamps and pauses the playhead when panning moves the viewport past it', () => {
    const { rerender } = render(
      <TimelineController durationSeconds={10} isDark={false} onZoomChange={vi.fn()} />
    );
    expect(mocks.setPlayheadTimeS).not.toHaveBeenCalled();
    expect(mocks.setPlayheadLineTimeMs).not.toHaveBeenCalled();

    mocks.zoomRange = { start: 2, end: 5 };
    rerender(<TimelineController durationSeconds={10} isDark={false} onZoomChange={vi.fn()} />);

    expect(mocks.setPlayheadTimeS).toHaveBeenCalledWith(5);
    expect(mocks.setPlayheadLineTimeMs).toHaveBeenCalledWith(5000);
    expect(mocks.setIsPlaying).toHaveBeenCalledWith(false);
  });
});
