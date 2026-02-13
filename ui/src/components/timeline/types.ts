export type TimelineSeries = Record<
  string,
  { binDuration: number; formatter: (value: number) => string; values: number[] }
>;

export const DEFAULT_TIMELINE_HEIGHT = 75;

// left/right spacing needs to be consistent across all timelines
// so axes line up. top/bottom spacing can be overridden, but defaults still
// provided here
export const TIMELINE_SPACING = {
  left: 50,
  right: 10,
  top: 5,
  bottom: 5,
};

// Timeline color constants live in useTimelineChartColors (canvas-based, theme mirrored in JS).
