import { useMemo } from 'react';
import { THEME_DARK, useTheme } from '@/contexts/ThemeContext';
import { BLACK, WHITE, withOpacity } from '@/services/colors';

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

/**
 * Theme-dependent colors for timeline ECharts (Timeline + TimelineController).
 * Centralizes all color constants and decisions so all timeline components stay in sync
 * This should be specific to timeline colors since they are canvas based and don't
 * benefit from the radix/shadcn component theming
 */
export function useTimelineChartColors() {
  const { theme } = useTheme();

  return useMemo(() => {
    const timelineMarkupColor =
      theme === THEME_DARK ? TIMELINE_MARKUP_COLOR_DARK : TIMELINE_MARKUP_COLOR;
    const gridBorderColor = withOpacity(timelineMarkupColor, GRID_BORDER_OPACITY);
    const dataZoomTextColor = theme === THEME_DARK ? WHITE : BLACK;
    const dataZoomTextBackgroundColor = withOpacity(
      theme === THEME_DARK ? BLACK : WHITE,
      DATAZOOM_LABEL_BACKGROUND_OPACITY
    );

    return {
      timelineMarkupColor,
      gridBorderColor,
      gridBackgroundColor: withOpacity(timelineMarkupColor, GRID_BACKGROUND_OPACITY),
      controllerGridBackgroundColor: withOpacity(
        timelineMarkupColor,
        CONTROLLER_GRID_BACKGROUND_OPACITY
      ),

      /* The root timeline data color */
      rollupTimelineColor:
        theme === THEME_DARK ? ROLLUP_TIMELINE_COLOR_DARK : ROLLUP_TIMELINE_COLOR,

      /* Datazoom options*/
      dataZoomTextColor,
      dataZoomTextBackgroundColor,
      dataZoomHandleColor: withOpacity(timelineMarkupColor, DATAZOOM_HANDLE_OPACITY),
      dataZoomFillerColor: withOpacity(timelineMarkupColor, DATAZOOM_FILLER_OPACITY),
      dataZoomEmphasisHandleColor: withOpacity(
        timelineMarkupColor,
        DATAZOOM_EMPHASIS_HANDLE_OPACITY
      ),
    };
  }, [theme]);
}
