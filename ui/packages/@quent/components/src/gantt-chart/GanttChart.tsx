// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect } from 'react';
import { useTimelinePointerPublisher, useTimelinePointerRatio } from '@quent/hooks';
import { useChartConnect } from '../lib/useChartConnect';
import { MIN_ZOOM_WINDOW_S } from '../lib/timeline.utils';
import { PlayheadLine } from '../timeline/PlayheadLine';
import { CHART_GROUP } from '../timeline/types';
import { GanttChartCore } from '../ui/charts/GanttChartCore';
import type { GanttChartCoreProps } from '../ui/charts/GanttChartCore';
import type { GanttDatum } from '../ui/charts/ganttOptions';

export type { GanttDatum, GanttGridSpacing, GanttRenderItem } from '../ui/charts/ganttOptions';

export interface GanttChartProps<T extends GanttDatum> extends Omit<
  GanttChartCoreProps<T>,
  | 'minZoomWindowSeconds'
  | 'pointerRatio'
  | 'onPointerRatioChange'
  | 'onChartReady'
  | 'renderOverlay'
> {
  showPlayhead?: boolean;
}

/** Hook-backed adapter for the controlled Gantt chart. */
export function GanttChart<T extends GanttDatum>({
  showPlayhead = true,
  ...props
}: GanttChartProps<T>) {
  const pointerRatio = useTimelinePointerRatio();
  const { publish, clear } = useTimelinePointerPublisher();
  const handlePointerRatioChange = useCallback(
    (ratio: number | null) => {
      if (ratio == null) {
        clear();
      } else {
        publish(ratio);
      }
    },
    [clear, publish]
  );
  const { handleChartReady, instanceRef } = useChartConnect({
    durationSeconds: props.durationSeconds,
    chartGroup: CHART_GROUP,
  });

  useEffect(
    () => () => {
      instanceRef.current = null;
    },
    [instanceRef]
  );

  return (
    <GanttChartCore
      {...props}
      minZoomWindowSeconds={MIN_ZOOM_WINDOW_S}
      pointerRatio={pointerRatio}
      onPointerRatioChange={handlePointerRatioChange}
      onChartReady={handleChartReady}
      renderOverlay={instance =>
        showPlayhead ? <PlayheadLine instance={instance} draggable /> : null
      }
    />
  );
}
