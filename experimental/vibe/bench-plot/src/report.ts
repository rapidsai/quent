// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

export type BenchLanguage = 'rust' | 'cpp' | 'python';

export interface BenchCase {
  implementation: string;
  language?: BenchLanguage;
  exporter: string | null;
  event_shape: string | null;
  threads: number;
  num_batches: number;
  batch_size: number;
  num_warmup_batches: number;
  batch_pause_interval_us: number;
  preflight_call: boolean;
  total_call_count?: number;
  discarded_call_count?: number | null;
  record_counts?: { attempted: number; observed: number } | null;
  thread_batch_elapsed_ns: number[][];
  average_ns_per_iteration: number;
}

export interface BenchReport {
  system: {
    captured_at_unix_seconds: number;
    os: string;
    architecture: string;
    cpu_model: string | null;
    available_cpu_count: number | null;
    build_profile: string | null;
    git_commit: string | null;
    git_dirty: boolean | null;
    rustc_version: string | null;
  };
  cases: BenchCase[];
}

export interface BoxSummary {
  low: number;
  q1: number;
  median: number;
  q3: number;
  high: number;
  outliers: number[];
}

const languageLabels: Record<BenchLanguage, string> = { rust: 'Rust', cpp: 'C++', python: 'Python' };

export function caseLanguage(value: BenchCase): BenchLanguage {
  return value.language ?? 'rust';
}

export function languageLabel(language: BenchLanguage): string {
  return languageLabels[language];
}

export function availableLanguages(report: BenchReport): BenchLanguage[] {
  const found = new Set(report.cases.map(caseLanguage));
  return (['rust', 'cpp', 'python'] as BenchLanguage[]).filter((language) => found.has(language));
}

export function caseLabel(value: BenchCase): string {
  if (value.event_shape === null) return `${languageLabel(caseLanguage(value))} / empty loop`;
  const framework = value.implementation === 'quent' ? 'Quent' : value.implementation;
  return `${framework} / ${value.exporter ?? 'default'}`;
}

export function discardedCallCount(value: BenchCase): number | null {
  if (value.discarded_call_count != null) return value.discarded_call_count;
  const counts = value.record_counts;
  return counts ? Math.max(0, counts.attempted - counts.observed) : null;
}

export function hasDiscardedCalls(value: BenchCase): boolean {
  return (discardedCallCount(value) ?? 0) > 0;
}

export function caseAxisLabel(value: BenchCase): string {
  return `${hasDiscardedCalls(value) ? '⚠ ' : ''}${caseLabel(value)}`;
}

export function isNoopCase(value: BenchCase): boolean {
  return value.total_call_count != null && value.total_call_count > 0 &&
    discardedCallCount(value) === value.total_call_count;
}

export function sortCasesByAverage(cases: BenchCase[]): BenchCase[] {
  return [...cases].sort((a, b) => a.average_ns_per_iteration - b.average_ns_per_iteration ||
    caseLabel(a).localeCompare(caseLabel(b)));
}

const fixedFrameworkColors: Record<string, string> = {
  quent: '#76B900',
  fastrace: '#332288',
  minitrace: '#44AA99',
  'slog-async': '#88CCEE',
  'spdlog-rs-async': '#DDCC77',
  ticklog: '#CC6677',
  tracing: '#AA4499',
  opentelemetry: '#882255',
  'empty-loop-rs': '#565656',
};

const extraColors = [
  '#F6222E', '#3283FE', '#FEAF16', '#B00068', '#1CFFCE', '#90AD1C',
  '#2ED9FF', '#DEA0FD', '#AA0DFE', '#F8A19F', '#325A9B', '#C4451C',
  '#1C8356', '#85660D', '#B10DA1', '#FBE426', '#1CBE4F', '#FA0087',
  '#FC1CBF', '#F7E1A0', '#C075A6', '#782AB6', '#BDCDFF', '#822E1C',
  '#B5EFB5', '#7ED7D1', '#1C7F93', '#D85FF7', '#683B79', '#66B0FF',
];

function generatedColor(index: number): string {
  const hue = (index * 137.508) % 360;
  const lightness = index % 2 ? 43 : 58;
  const saturation = 65;
  const channel = (offset: number): string => {
    const k = (offset + hue / 30) % 12;
    const a = saturation * Math.min(lightness, 100 - lightness) / 100;
    return Math.round((lightness - a * Math.max(-1, Math.min(k - 3, 9 - k, 1))) * 2.55)
      .toString(16).padStart(2, '0');
  };
  return `#${channel(0)}${channel(8)}${channel(4)}`;
}

export function frameworkColors(implementations: Iterable<string>): ReadonlyMap<string, string> {
  const colors = new Map<string, string>();
  const used = new Set(Object.values(fixedFrameworkColors).map((color) => color.toLowerCase()));
  let next = 0;
  for (const implementation of [...new Set(implementations)].sort()) {
    if (Object.hasOwn(fixedFrameworkColors, implementation)) {
      colors.set(implementation, fixedFrameworkColors[implementation]);
      continue;
    }
    let color: string;
    do {
      color = extraColors[next] ?? generatedColor(next - extraColors.length);
      next++;
    } while (used.has(color.toLowerCase()));
    colors.set(implementation, color);
    used.add(color.toLowerCase());
  }
  return colors;
}

export function frameworkColor(implementation: string, colors: ReadonlyMap<string, string>, lightness?: number,
  saturationScale = 1): string {
  const color = colors.get(implementation);
  if (!color) throw new Error(`Missing color for ${implementation}`);
  if (lightness === undefined && saturationScale === 1) return color;
  const [red, green, blue] = [1, 3, 5].map((position) => parseInt(color.slice(position, position + 2), 16) / 255);
  const high = Math.max(red, green, blue);
  const low = Math.min(red, green, blue);
  const delta = high - low;
  const baseLightness = (high + low) / 2;
  const saturation = delta === 0 ? 0 : delta / (1 - Math.abs(2 * baseLightness - 1));
  let hue = 0;
  if (delta > 0) {
    if (high === red) hue = ((green - blue) / delta) % 6;
    else if (high === green) hue = (blue - red) / delta + 2;
    else hue = (red - green) / delta + 4;
  }
  return `hsl(${Math.round((hue + 6) % 6 * 60)}, ${Math.round(saturation * 100 * saturationScale)}%, ${lightness ?? Math.round(baseLightness * 100)}%)`;
}

export function batchAverages(value: BenchCase): number[] {
  return Array.from({ length: value.num_batches }, (_, batch) =>
    value.thread_batch_elapsed_ns.reduce((sum, thread) => sum + thread[batch], 0) /
    value.threads / value.batch_size);
}

export function nearestRank(sorted: number[], percentage: number): number {
  return sorted[Math.max(0, Math.ceil(sorted.length * percentage / 100) - 1)];
}

export function boxSummary(value: BenchCase): BoxSummary {
  const sorted = batchAverages(value).sort((a, b) => a - b);
  const q1 = nearestRank(sorted, 25);
  const median = nearestRank(sorted, 50);
  const q3 = nearestRank(sorted, 75);
  const fenceLow = q1 - 1.5 * (q3 - q1);
  const fenceHigh = q3 + 1.5 * (q3 - q1);
  return {
    low: sorted.find((sample) => sample >= fenceLow)!,
    q1,
    median,
    q3,
    high: sorted.findLast((sample) => sample <= fenceHigh)!,
    outliers: sorted.filter((sample) => sample < fenceLow || sample > fenceHigh),
  };
}

export function payloadGroups(report: BenchReport, language: BenchLanguage = 'rust', includeNoop = true,
  includeDiscarded = true): Array<{ name: string; threads: number[] }> {
  const groups = new Map<string, Set<number>>();
  for (const item of report.cases) {
    if (caseLanguage(item) !== language || item.event_shape === null ||
      (!includeNoop && isNoopCase(item)) || (!includeDiscarded && hasDiscardedCalls(item))) continue;
    if (!groups.has(item.event_shape)) groups.set(item.event_shape, new Set());
    groups.get(item.event_shape)!.add(item.threads);
  }
  const preferred = ['empty', 'u8', 'u64', 'short-string', 'long-string', 'all'];
  return [...groups].sort(([a], [b]) => {
    const aIndex = preferred.indexOf(a);
    const bIndex = preferred.indexOf(b);
    return (aIndex < 0 ? preferred.length : aIndex) - (bIndex < 0 ? preferred.length : bIndex) || a.localeCompare(b);
  }).map(([name, threads]) => ({ name, threads: [...threads].sort((a, b) => a - b) }));
}
