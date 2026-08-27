// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { EChartsInstance } from 'echarts-for-react';
import { useDataFlowIsPlaying } from '@quent/hooks';
import { cn } from '@quent/utils';
import { usePlayheadLinePixel } from '../lib/usePlayheadLinePixel';

type PlayheadLineProps = {
  instance: EChartsInstance | null;
  xAxisIndex?: number;
};

/** Playhead overlay aligned to an ECharts x-axis. */
export function PlayheadLine({ instance, xAxisIndex = 0 }: PlayheadLineProps) {
  const pixelX = usePlayheadLinePixel(instance, xAxisIndex);
  const isPlaying = useDataFlowIsPlaying();

  if (pixelX == null) {
    return null;
  }

  return (
    <div
      className={cn(
        'pointer-events-none absolute bottom-0 top-0 z-[10] w-px bg-primary/70',
        isPlaying && 'transition-[left] duration-100 ease-linear motion-reduce:transition-none'
      )}
      style={{ left: pixelX }}
    />
  );
}
