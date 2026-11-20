// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback } from 'react';
import type { EChartsInstance } from 'echarts-for-react';
import { useTimelinePointerPublisher, useTimelinePointerRatio, useZoomRange } from '@quent/hooks';
import { useChartConnect } from '../lib/useChartConnect';
import { MIN_ZOOM_WINDOW_S } from '../lib/timeline.utils';
import { TimelineChart } from '../ui/charts/TimelineChart';
import type { TimelineHoverPosition } from '../ui/charts/TimelineChart';
import type { TimelineMark, TimelineSeries } from './types';
import { CHART_GROUP } from './types';

export type { TimelineHoverPosition } from '../ui/charts/TimelineChart';

export interface TimelineProps {
  durationSeconds: number;
  series: TimelineSeries;
  timestamps: number[];
  showTooltip?: boolean;
  marks?: TimelineMark[];
  isDark: boolean;
  yAxisLabel?: string | null;
  onHoverChange?: (position: TimelineHoverPosition | null) => void;
  onReady?: (instance: EChartsInstance) => void;
}

/** Hook-backed adapter for the controlled timeline chart. */
export function Timeline({ marks, ...props }: TimelineProps) {
  const chartMarks = marks?.map(({ label, stateName, operatorName, ...mark }) => ({
    ...mark,
    labelLines: [label, stateName, ...(operatorName ? [operatorName] : [])],
  }));
  const zoomRange = useZoomRange();
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
  const { handleChartReady } = useChartConnect({
    durationSeconds: props.durationSeconds,
    chartGroup: CHART_GROUP,
  });

  return (
    <TimelineChart
      {...props}
      marks={chartMarks}
      visibleStartMs={zoomRange.start * 1_000}
      visibleEndMs={zoomRange.end * 1_000}
      minZoomWindowSeconds={MIN_ZOOM_WINDOW_S}
      pointerRatio={pointerRatio}
      onPointerRatioChange={handlePointerRatioChange}
      onChartReady={handleChartReady}
    />
  );
}
