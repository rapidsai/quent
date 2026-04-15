// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo } from 'react';
import { BLACK, WHITE, withOpacity } from '@quent/utils';

// Timeline chart colors
const TIMELINE_MARKUP_COLOR = '#808080';
const TIMELINE_MARKUP_COLOR_DARK = '#A0A0A0';
const ROLLUP_TIMELINE_COLOR = '#AAAAAA';
const ROLLUP_TIMELINE_COLOR_DARK = '#777777';

const GRID_BORDER_OPACITY = 0.2;
const GRID_BACKGROUND_OPACITY = 0.1;
const CONTROLLER_GRID_BACKGROUND_OPACITY = 0.05;
const DATAZOOM_HANDLE_OPACITY = 0.3;
const DATAZOOM_FILLER_OPACITY = 0.2;
const DATAZOOM_EMPHASIS_HANDLE_OPACITY = 0.5;
const DATAZOOM_LABEL_BACKGROUND_OPACITY = 0.5;
const OVERLAY_LIGHTEN = 0.6;
const OVERLAY_LIGHTEN_DARK = 0.4;

export const TIMELINE_MONO_FONT =
  'ui-monospace, SFMono-Regular, SF Mono, Menlo, Consolas, Liberation Mono, monospace';

const MARK_AREA_FILL_OPACITY = 0.12;
const MARK_AREA_BORDER_OPACITY = 0.75;
const MARK_LABEL_TEXT_COLOR = WHITE;
const MARK_LABEL_TEXT_COLOR_DARK = WHITE;

/** Softer than pure black/white for chart text to reduce contrast. */
const TEXT_COLOR_LIGHT = '#333333';
const TEXT_COLOR_DARK = '#d4d4d4';

/**
 * Theme-dependent colors for timeline ECharts (Timeline + TimelineController).
 * Centralizes all color constants and decisions so all timeline components stay in sync.
 * This should be specific to timeline colors since they are canvas based and don't
 * benefit from the radix/shadcn component theming.
 *
 * @param isDark - Whether the dark theme is active. Callers obtain this from their own
 *   theme context or prop rather than importing ThemeContext directly, keeping this
 *   component decoupled from any specific theme provider.
 */
export function useTimelineChartColors(isDark: boolean) {
  return useMemo(() => {
    const timelineMarkupColor = isDark ? TIMELINE_MARKUP_COLOR_DARK : TIMELINE_MARKUP_COLOR;
    const gridBorderColor = withOpacity(timelineMarkupColor, GRID_BORDER_OPACITY);
    const dataZoomTextColor = isDark ? WHITE : BLACK;
    const dataZoomTextBackgroundColor = withOpacity(
      isDark ? BLACK : WHITE,
      DATAZOOM_LABEL_BACKGROUND_OPACITY
    );

    const textColor = isDark ? TEXT_COLOR_DARK : TEXT_COLOR_LIGHT;

    return {
      textColor,
      timelineMarkupColor,
      gridBorderColor,
      gridBackgroundColor: withOpacity(timelineMarkupColor, GRID_BACKGROUND_OPACITY),
      controllerGridBackgroundColor: withOpacity(
        timelineMarkupColor,
        CONTROLLER_GRID_BACKGROUND_OPACITY
      ),

      /* The root timeline data color */
      rollupTimelineColor: isDark ? ROLLUP_TIMELINE_COLOR_DARK : ROLLUP_TIMELINE_COLOR,

      /* Datazoom options*/
      dataZoomTextColor,
      dataZoomTextBackgroundColor,
      dataZoomHandleColor: withOpacity(timelineMarkupColor, DATAZOOM_HANDLE_OPACITY),
      dataZoomFillerColor: withOpacity(timelineMarkupColor, DATAZOOM_FILLER_OPACITY),
      dataZoomEmphasisHandleColor: withOpacity(
        timelineMarkupColor,
        DATAZOOM_EMPHASIS_HANDLE_OPACITY
      ),

      overlayLighten: isDark ? OVERLAY_LIGHTEN_DARK : OVERLAY_LIGHTEN,

      markAreaFillOpacity: MARK_AREA_FILL_OPACITY,
      markAreaBorderOpacity: MARK_AREA_BORDER_OPACITY,
      markLabelTextColor: isDark ? MARK_LABEL_TEXT_COLOR_DARK : MARK_LABEL_TEXT_COLOR,
    };
  }, [isDark]);
}
