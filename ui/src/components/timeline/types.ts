export type TimelineSeriesEntry = {
  binDuration: number;
  formatter: (value: number, decimals?: number) => string;
  values: number[];
  color: string;
  isOverlay?: boolean;
  /** When true, this series is dimmed to make overlay series stand out. */
  isDimmed?: boolean;
};

export type TimelineSeries = Record<string, TimelineSeriesEntry>;

/** A single annotation mark on the timeline. */
export type TimelineMark = {
  label: string;
  stateName: string;
  color: string;
  xStart: number;
  xEnd: number;
  /** When true, this mark is dimmed (e.g. not part of the selected operator's long entities). */
  isDimmed?: boolean;
  /** Operator instance name when this mark belongs to a selected operator's long entities. */
  operatorName?: string;
};

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

// Shared axis animation settings for timeline charts.
export const TIMELINE_X_AXIS_ANIMATION = {
  animation: false,
  animationDuration: 50,
  animationDurationUpdate: 100,
  animationEasingUpdate: 'cubicOut',
} as const;
