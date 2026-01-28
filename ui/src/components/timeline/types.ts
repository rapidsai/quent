export type TimelineSeries = Record<
  string,
  { binDuration: number; formatter: (value: number) => string; values: number[] }
>;

export const DEFAULT_TIMELINE_HEIGHT = 100;
