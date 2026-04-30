// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo } from 'react';
import { echarts } from '@/lib/echarts';
import { THEME_DARK, useTheme } from '@/contexts/ThemeContext';
import { BLACK, WHITE, withOpacity } from '@/services/colors';

/**
 * Centralizes ECharts styling for the timeline charts (Timeline,
 * TimelineController, OperatorGanttChart) by registering two themes
 * (`quent-timeline-light` / `quent-timeline-dark`) at module load.
 *
 * Components opt in by passing `theme={themeName}` to ReactECharts,
 * which lets us drop most of the inline color/axis/dataZoom configuration
 * and rely on theme defaults instead.
 *
 * Constants and the small `useTimelineEchartsTheme` hook below cover the
 * few values that still need to be referenced from JS — primarily because
 * they are consumed inside custom series renderers or per-series options
 * that the ECharts theme system can't express.
 */

export const TIMELINE_MONO_FONT =
  'ui-monospace, SFMono-Regular, SF Mono, Menlo, Consolas, Liberation Mono, monospace';

export const MARK_AREA_FILL_OPACITY = 0.12;
export const MARK_AREA_BORDER_OPACITY = 0.75;
/** Mark labels sit on a colored chip; white reads well against every state color. */
export const MARK_LABEL_TEXT_COLOR = WHITE;

export const TIMELINE_THEME_NAME_LIGHT = 'quent-timeline-light';
export const TIMELINE_THEME_NAME_DARK = 'quent-timeline-dark';

const TIMELINE_MARKUP_COLOR_LIGHT = '#808080';
const TIMELINE_MARKUP_COLOR_DARK = '#A0A0A0';
export const ROLLUP_TIMELINE_COLOR_LIGHT = '#AAAAAA';
export const ROLLUP_TIMELINE_COLOR_DARK = '#777777';

/** Softer than pure black/white for chart text to reduce contrast. */
const TEXT_COLOR_LIGHT = '#333333';
const TEXT_COLOR_DARK = '#d4d4d4';

const GRID_BORDER_OPACITY = 0.2;
const GRID_BACKGROUND_OPACITY = 0.1;
const CONTROLLER_GRID_BACKGROUND_OPACITY = 0.05;
const DATAZOOM_HANDLE_OPACITY = 0.3;
const DATAZOOM_FILLER_OPACITY = 0.2;
const DATAZOOM_EMPHASIS_HANDLE_OPACITY = 0.5;
const DATAZOOM_LABEL_BACKGROUND_OPACITY = 0.5;

function buildTimelineTheme(isDark: boolean) {
  const timelineMarkupColor = isDark ? TIMELINE_MARKUP_COLOR_DARK : TIMELINE_MARKUP_COLOR_LIGHT;
  const rollupTimelineColor = isDark ? ROLLUP_TIMELINE_COLOR_DARK : ROLLUP_TIMELINE_COLOR_LIGHT;
  const textColor = isDark ? TEXT_COLOR_DARK : TEXT_COLOR_LIGHT;
  const gridBorderColor = withOpacity(timelineMarkupColor, GRID_BORDER_OPACITY);
  const gridBackgroundColor = withOpacity(timelineMarkupColor, GRID_BACKGROUND_OPACITY);

  const dataZoomTextColor = isDark ? WHITE : BLACK;
  const dataZoomTextBackgroundColor = withOpacity(
    isDark ? BLACK : WHITE,
    DATAZOOM_LABEL_BACKGROUND_OPACITY
  );
  const dataZoomHandleColor = withOpacity(timelineMarkupColor, DATAZOOM_HANDLE_OPACITY);
  const dataZoomFillerColor = withOpacity(timelineMarkupColor, DATAZOOM_FILLER_OPACITY);
  const dataZoomEmphasisHandleColor = withOpacity(
    timelineMarkupColor,
    DATAZOOM_EMPHASIS_HANDLE_OPACITY
  );

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
    axisPointer: {
      lineStyle: { type: 'dashed', color: timelineMarkupColor },
    },
  };

  return {
    // Default series color palette. Series that don't specify their own color
    // (e.g. TimelineController.static-display) inherit this.
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
      handleStyle: { color: dataZoomHandleColor, width: 2 },
      fillerColor: dataZoomFillerColor,
      borderColor: 'transparent',
      backgroundColor: 'transparent',
      moveHandleSize: 5,
      dataBackground: { lineStyle: { opacity: 0 }, areaStyle: { opacity: 0 } },
      selectedDataBackground: { lineStyle: { opacity: 0 }, areaStyle: { opacity: 0 } },
      textStyle: {
        color: dataZoomTextColor,
        backgroundColor: dataZoomTextBackgroundColor,
        padding: [2, 4],
        borderRadius: 2,
        fontFamily: TIMELINE_MONO_FONT,
      },
      emphasis: {
        handleStyle: { color: dataZoomEmphasisHandleColor },
      },
    },
  };
}

echarts.registerTheme(TIMELINE_THEME_NAME_LIGHT, buildTimelineTheme(false));
echarts.registerTheme(TIMELINE_THEME_NAME_DARK, buildTimelineTheme(true));

/**
 * Returns the registered timeline theme name plus the small set of theme-derived
 * values that the registered theme can't express (custom series rendering,
 * series-specific overrides, etc.).
 */
export function useTimelineEchartsTheme() {
  const { theme } = useTheme();
  const isDark = theme === THEME_DARK;

  return useMemo(
    () => ({
      themeName: isDark ? TIMELINE_THEME_NAME_DARK : TIMELINE_THEME_NAME_LIGHT,
      textColor: isDark ? TEXT_COLOR_DARK : TEXT_COLOR_LIGHT,
      controllerGridBackgroundColor: withOpacity(
        isDark ? TIMELINE_MARKUP_COLOR_DARK : TIMELINE_MARKUP_COLOR_LIGHT,
        CONTROLLER_GRID_BACKGROUND_OPACITY
      ),
    }),
    [isDark]
  );
}
