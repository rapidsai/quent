// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { EChartsInstance } from 'echarts-for-react';
import { usePlayheadLinePixel } from '../lib/usePlayheadLinePixel';

type PlayheadLineProps = {
  instance: EChartsInstance | null;
  xAxisIndex?: number;
};

/** Playhead overlay aligned to an ECharts x-axis. */
export function PlayheadLine({ instance, xAxisIndex = 0 }: PlayheadLineProps) {
  const pixelX = usePlayheadLinePixel(instance, xAxisIndex);

  if (pixelX == null) return null;

  return (
    <div
      className="absolute top-0 bottom-0 w-px pointer-events-none z-[10] bg-primary/70"
      style={{ left: pixelX }}
    />
  );
}
