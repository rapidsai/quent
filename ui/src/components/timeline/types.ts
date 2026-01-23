export type TimelineSeries = Record<
  string,
  { binDuration: number; formatter: (value: number) => string; values: number[] }
>;
