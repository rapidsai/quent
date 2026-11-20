// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo } from 'react';
import { echarts } from './echarts';
import { withOpacity } from '../colors';

const BLACK = '#000000';
const WHITE = '#ffffff';

export const TIMELINE_MONO_FONT =
  'ui-monospace, SFMono-Regular, SF Mono, Menlo, Consolas, Liberation Mono, monospace';
export const TIMELINE_LABEL_FONT_SIZE = 10;
export const MARK_AREA_FILL_OPACITY = 0.12;
export const MARK_AREA_BORDER_OPACITY = 0.75;
export const DIMMED_OPACITY = 0.25;
export const MARK_LABEL_TEXT_COLOR = WHITE;
export const TIMELINE_THEME_NAME_LIGHT = 'quent-timeline-light';
export const TIMELINE_THEME_NAME_DARK = 'quent-timeline-dark';

const TIMELINE_MARKUP_COLOR_LIGHT = '#808080';
const TIMELINE_MARKUP_COLOR_DARK = '#A0A0A0';
export const ROLLUP_TIMELINE_COLOR_LIGHT = '#AAAAAA';
export const ROLLUP_TIMELINE_COLOR_DARK = '#777777';
const TEXT_COLOR_LIGHT = '#333333';
const TEXT_COLOR_DARK = '#d4d4d4';
const AXIS_TICK_COLOR_LIGHT = '#aaaaaa';
const AXIS_TICK_COLOR_DARK = '#c0c0c0';
const GRID_BORDER_OPACITY = 0.2;
const GRID_BACKGROUND_OPACITY = 0.1;
const CONTROLLER_GRID_BACKGROUND_OPACITY = 0.05;
const DATAZOOM_HANDLE_OPACITY = 0.3;
const DATAZOOM_FILLER_OPACITY = 0.2;
const DATAZOOM_EMPHASIS_HANDLE_OPACITY = 0.5;

function buildTimelineTheme(isDark: boolean) {
  const timelineMarkupColor = isDark ? TIMELINE_MARKUP_COLOR_DARK : TIMELINE_MARKUP_COLOR_LIGHT;
  const rollupTimelineColor = isDark ? ROLLUP_TIMELINE_COLOR_DARK : ROLLUP_TIMELINE_COLOR_LIGHT;
  const textColor = isDark ? TEXT_COLOR_DARK : TEXT_COLOR_LIGHT;
  const gridBorderColor = withOpacity(timelineMarkupColor, GRID_BORDER_OPACITY);
  const gridBackgroundColor = withOpacity(timelineMarkupColor, GRID_BACKGROUND_OPACITY);
  const sharedAxis = {
    axisLine: { show: true, lineStyle: { color: gridBorderColor } },
    axisTick: { show: false },
    splitLine: { show: false },
    axisLabel: {
      show: true,
      color: timelineMarkupColor,
      fontSize: 10,
      fontFamily: TIMELINE_MONO_FONT,
      margin: 8,
    },
    axisPointer: { lineStyle: { type: 'dashed', color: timelineMarkupColor } },
  };

  return {
    color: [rollupTimelineColor],
    backgroundColor: 'transparent',
    textStyle: { color: textColor, fontFamily: TIMELINE_MONO_FONT },
    grid: {
      backgroundColor: gridBackgroundColor,
      borderColor: gridBorderColor,
      borderWidth: 1,
      show: true,
    },
    valueAxis: sharedAxis,
    timeAxis: sharedAxis,
    categoryAxis: sharedAxis,
    logAxis: sharedAxis,
    dataZoom: {
      handleStyle: {
        color: withOpacity(timelineMarkupColor, DATAZOOM_HANDLE_OPACITY),
        width: 2,
      },
      fillerColor: withOpacity(timelineMarkupColor, DATAZOOM_FILLER_OPACITY),
      borderColor: 'transparent',
      backgroundColor: 'transparent',
      moveHandleSize: 5,
      dataBackground: { lineStyle: { opacity: 0 }, areaStyle: { opacity: 0 } },
      selectedDataBackground: { lineStyle: { opacity: 0 }, areaStyle: { opacity: 0 } },
      emphasis: {
        handleStyle: {
          color: withOpacity(timelineMarkupColor, DATAZOOM_EMPHASIS_HANDLE_OPACITY),
        },
      },
    },
  };
}

echarts.registerTheme(TIMELINE_THEME_NAME_LIGHT, buildTimelineTheme(false));
echarts.registerTheme(TIMELINE_THEME_NAME_DARK, buildTimelineTheme(true));

/** Registered chart theme and values needed by DOM/custom-series overlays. */
export function useTimelineEchartsTheme(isDark: boolean) {
  return useMemo(
    () => ({
      themeName: isDark ? TIMELINE_THEME_NAME_DARK : TIMELINE_THEME_NAME_LIGHT,
      textColor: isDark ? TEXT_COLOR_DARK : TEXT_COLOR_LIGHT,
      axisLabelColor: isDark ? TIMELINE_MARKUP_COLOR_DARK : TIMELINE_MARKUP_COLOR_LIGHT,
      axisTickColor: isDark ? AXIS_TICK_COLOR_DARK : AXIS_TICK_COLOR_LIGHT,
      labelBackgroundColor: withOpacity(isDark ? BLACK : WHITE, 0.75),
      solidLabelBackgroundColor: withOpacity(isDark ? BLACK : WHITE, 0.95),
      controllerGridBackgroundColor: withOpacity(
        isDark ? TIMELINE_MARKUP_COLOR_DARK : TIMELINE_MARKUP_COLOR_LIGHT,
        CONTROLLER_GRID_BACKGROUND_OPACITY
      ),
    }),
    [isDark]
  );
}
