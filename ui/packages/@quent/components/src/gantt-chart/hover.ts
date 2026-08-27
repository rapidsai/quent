// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { EChartsInstance } from 'echarts-for-react';

export interface GanttHover {
  timestampMs: number;
  clientX: number;
  clientY: number;
}

/** Observe pointer movement inside an ECharts grid and report its timestamp. */
export function observeGanttHover(
  instance: EChartsInstance,
  onChange: (hover: GanttHover | null) => void
): () => void {
  const dom = instance.getDom();
  const onPointerMove = (event: PointerEvent) => {
    if (instance.isDisposed?.()) {
      return;
    }
    const rect = dom.getBoundingClientRect();
    const point: [number, number] = [event.clientX - rect.left, event.clientY - rect.top];
    if (!instance.containPixel({ gridIndex: 0 }, point)) {
      onChange(null);
      return;
    }
    try {
      const value = instance.convertFromPixel({ xAxisIndex: 0 }, point[0]);
      if (value == null || !Number.isFinite(value as number)) {
        return;
      }
      onChange({
        timestampMs: value as number,
        clientX: event.clientX,
        clientY: event.clientY,
      });
    } catch {
      onChange(null);
    }
  };
  const onPointerLeave = () => onChange(null);

  dom.addEventListener('pointermove', onPointerMove);
  dom.addEventListener('pointerleave', onPointerLeave);
  dom.addEventListener('pointercancel', onPointerLeave);

  return () => {
    dom.removeEventListener('pointermove', onPointerMove);
    dom.removeEventListener('pointerleave', onPointerLeave);
    dom.removeEventListener('pointercancel', onPointerLeave);
  };
}
