// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo } from 'react';
import ReactEChartsComponent from 'echarts-for-react';
import { echarts } from '../lib/echarts';
import type { EChartsOption } from '../lib/echarts';
import { useZoomRange } from '@quent/hooks';
import { formatDurationForAxisInterval } from '@quent/utils';
import { nanosToMs, getTimelineXAxisIntervalMs, MIN_ZOOM_WINDOW_S } from '../lib/timeline.utils';
import { useChartResize } from '../lib/useChartResize';
import {
  useTimelineEchartsTheme,
  TIMELINE_MONO_FONT,
  TIMELINE_LABEL_FONT_SIZE,
} from './timelineEchartsTheme';
import { TIMELINE_SPACING } from './types';

const RULER_HEIGHT = 22;
const RULER_TARGET_TICKS = 7;
// Space above the grid for axis labels + ticks.
const RULER_GRID_TOP = 20;

/**
 * `absolute`: elapsed time from query start (e.g. "20.00ms", "40.00ms").
 * `relative`: offset from the start of the current zoom window, always begins near 0 (e.g. "+0.00ms", "+20.00µs").
 */
export type TimelineRulerMode = 'absolute' | 'relative';

type TimelineRulerProps = {
  startTime: bigint;
  isDark: boolean;
  mode?: TimelineRulerMode;
};

/** Sticky axis ruler showing elapsed time for the current zoom window. */
export function TimelineRuler({ startTime, isDark, mode = 'relative' }: TimelineRulerProps) {
  const { themeName, axisTickColor, axisLabelColor, solidLabelBackgroundColor } =
    useTimelineEchartsTheme(isDark);
  const { handleChartReady } = useChartResize();
  const startTimeMs = useMemo(() => nanosToMs(startTime), [startTime]);
  const zoomRange = useZoomRange();

  const zoomedStartMs = startTimeMs + zoomRange.start * 1000;
  const zoomedEndMs = startTimeMs + zoomRange.end * 1000;
  const zoomedSpanMs = Math.max(zoomedEndMs - zoomedStartMs, MIN_ZOOM_WINDOW_S * 1000);

  const interval = useMemo(
    () => getTimelineXAxisIntervalMs(zoomedSpanMs, RULER_TARGET_TICKS),
    [zoomedSpanMs]
  );

  const option: EChartsOption = useMemo(() => {
    const formatLabel = (value: number): string => {
      const absoluteMs = value - startTimeMs;
      const relativeMs = value - zoomedStartMs;
      const isMinMax = value === zoomedStartMs || value === zoomedEndMs;

      let text: string;
      if (mode === 'relative') {
        const relStr = `+${formatDurationForAxisInterval(relativeMs, interval)}`;
        text = isMinMax ? `${formatDurationForAxisInterval(absoluteMs, interval)}` : relStr;
      } else {
        text = formatDurationForAxisInterval(absoluteMs, interval);
      }

      // Wrap min/max labels in the datazoom-chip rich style.
      return isMinMax ? `{chip|${text}}` : text;
    };

    return {
      animation: false,
      grid: {
        ...TIMELINE_SPACING,
        left: 1,
        top: RULER_GRID_TOP,
        bottom: 0,
      },
      xAxis: {
        type: 'value',
        show: true,
        position: 'top',
        min: zoomedStartMs,
        max: zoomedEndMs,
        interval,
        boundaryGap: false,
        // Re-enable ticks (theme disables them by default); prominent color + extra length.
        axisTick: {
          show: true,
          alignWithLabel: true,
          length: 8,
          lineStyle: { color: axisTickColor },
        },
        axisLabel: {
          hideOverlap: true,
          showMinLabel: true,
          showMaxLabel: true,
          alignMinLabel: 'left',
          alignMaxLabel: 'right',
          formatter: formatLabel,
          rich: {
            chip: {
              color: axisLabelColor,
              backgroundColor: solidLabelBackgroundColor,
              borderColor: axisLabelColor,
              borderWidth: 1,
              borderRadius: 2,
              padding: [1, 4, 1, 4],
              fontSize: TIMELINE_LABEL_FONT_SIZE,
              fontFamily: TIMELINE_MONO_FONT,
              lineHeight: TIMELINE_LABEL_FONT_SIZE,
            },
          },
        },
        splitLine: { show: false },
        axisPointer: { show: false },
      },
      yAxis: { type: 'value', show: false, min: 0, max: 1 },
      // Dummy series to give echarts a data domain to anchor the axis.
      series: [
        {
          type: 'line',
          data: [
            [zoomedStartMs, 0],
            [zoomedEndMs, 0],
          ],
          lineStyle: { opacity: 0 },
          symbol: 'none',
          silent: true,
          animation: false,
        },
      ],
    };
  }, [
    zoomedStartMs,
    zoomedEndMs,
    interval,
    startTimeMs,
    axisTickColor,
    axisLabelColor,
    solidLabelBackgroundColor,
    mode,
  ]);

  return (
    <ReactEChartsComponent
      echarts={echarts}
      theme={themeName}
      option={option}
      style={{ width: '100%', height: `${RULER_HEIGHT}px` }}
      onChartReady={handleChartReady}
      notMerge={false}
      lazyUpdate={false}
      opts={{ renderer: 'svg' }}
      autoResize={false}
    />
  );
}
