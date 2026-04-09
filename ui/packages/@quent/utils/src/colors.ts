/**
 * Centralized color palette and mapping utilities for charts and visualizations.
 */

import type { FsmTypeDecl } from './types';

/**
 * Available color palettes for charts.
 */
export const PALETTES = {
  /** Wong colorblind-friendly palette - optimized for accessibility */
  wong: [
    '#0072B2', // Blue
    '#E69F00', // Orange
    '#009E73', // Teal
    '#F0E442', // Yellow
    '#56B4E9', // Sky Blue
    '#D55E00', // Vermillion
    '#CC79A7', // Pink
  ],
  /** Default ECharts palette */
  echarts: [
    '#5470c6', // Blue
    '#91cc75', // Green
    '#fac858', // Yellow
    '#ee6666', // Red
    '#73c0de', // Light Blue
    '#3ba272', // Teal
    '#fc8452', // Orange
    '#9a60b4', // Purple
    '#ea7ccc', // Pink
  ],
  /** Tol qualitative colorblind-friendly palette */
  extended: [
    '#44AA99', // Teal
    '#CC6677', // Rose
    '#332288', // Indigo
    '#DDCC77', // Sand
    '#AA4499', // Purple
    '#88CCEE', // Cyan
    '#882255', // Wine
    '#88AA55', // Muted Lime
    '#666666', // Grey
  ],
} as const;

export type PaletteName = keyof typeof PALETTES;
export type ChartColor = string;

// Current active palette
let activePalette: PaletteName = 'extended';

/**
 * Get the currently active palette.
 */
export function getActivePalette(): readonly string[] {
  return PALETTES[activePalette];
}

/**
 * Set the active palette by name.
 */
export function setActivePalette(name: PaletteName): void {
  activePalette = name;
  resetColorAssignments();
}

/**
 * Get palette by name.
 */
export function getPalette(name: PaletteName): readonly string[] {
  return PALETTES[name];
}

/**
 * Simple string hash function (djb2 algorithm).
 * Returns a positive integer hash for the given string.
 */
function hashString(str: string): number {
  let hash = 5381;
  for (let i = 0; i < str.length; i++) {
    hash = (hash * 33) ^ str.charCodeAt(i);
  }
  return hash >>> 0; // Convert to unsigned 32-bit integer
}

// Cache: key -> palette index
const colorAssignments = new Map<string, number>();
// Track which palette indices are taken
const usedIndices = new Set<number>();

/**
 * Get a deterministic color for a given key.
 * Uses a hash to pick a starting index, then probes forward to avoid
 * collisions so different keys get different colors (until the palette
 * is exhausted, after which duplicates are allowed).
 */
export function getColorForKey(key: string): ChartColor {
  const palette = getActivePalette();

  if (colorAssignments.has(key)) {
    return palette[colorAssignments.get(key)!];
  }

  const hashIndex = hashString(key) % palette.length;

  // If palette is full, just use the hash index
  if (usedIndices.size >= palette.length) {
    colorAssignments.set(key, hashIndex);
    return palette[hashIndex];
  }

  // Probe forward from hash index to find an unused slot
  let index = hashIndex;
  while (usedIndices.has(index)) {
    index = (index + 1) % palette.length;
  }

  colorAssignments.set(key, index);
  usedIndices.add(index);
  return palette[index];
}

/**
 * Assign colors to an array of keys in order.
 * Useful for batch assignment to maintain consistent ordering.
 */
export function assignColors<T extends string>(keys: T[]): Record<T, ChartColor> {
  const palette = getActivePalette();
  return Object.fromEntries(
    keys.map((key, index) => [key, palette[index % palette.length]])
  ) as Record<T, ChartColor>;
}

/**
 * Create a capacity->color resolver for timeline capacity series.
 * Multiple capacities use ordered palette assignment; a single capacity uses
 * key-based deterministic coloring to stay stable across timelines.
 */
export function createCapacitiesColorFn(
  capacityKeys: string[]
): (capacityName: string) => ChartColor {
  const colorMap =
    capacityKeys.length > 1
      ? assignColors(capacityKeys)
      : Object.fromEntries(capacityKeys.map(capacity => [capacity, getColorForKey(capacity)]));

  return (capacityName: string) => colorMap[capacityName] ?? getColorForKey(capacityName);
}

/**
 * Get a color by index from the active palette (wraps around).
 */
export function getColorByIndex(index: number): ChartColor {
  const palette = getActivePalette();
  return palette[index % palette.length];
}

export function createFsmTypeColorFn(fsmTypes: { [key in string]?: FsmTypeDecl }): (
  stateName: string
) => ChartColor {
  const stateIndexMap = buildFsmStateIndexMap(fsmTypes);
  return (stateName: string) => {
    const stateIndex = stateIndexMap.get(stateName);
    return stateIndex != null ? getColorByIndex(stateIndex) : getColorForKey(stateName);
  };
}

/**
 * Build a deterministic state->index lookup from FSM declarations.
 * State index controls palette position so same state names stay consistent.
 */
function buildFsmStateIndexMap(fsmTypes?: { [key in string]?: FsmTypeDecl }): Map<string, number> {
  const stateIndexMap = new Map<string, number>();
  if (!fsmTypes) return stateIndexMap;

  for (const decl of Object.values(fsmTypes)) {
    if (!decl) continue;
    for (let i = 0; i < decl.states.length; i++) {
      stateIndexMap.set(decl.states[i]!.name, i);
    }
  }

  return stateIndexMap;
}

/**
 * Add opacity to a hex color.
 * @param hex - Hex color string (e.g., '#0072B2')
 * @param opacity - Opacity value between 0 and 1
 * @returns Hex color with alpha (e.g., '#0072B2CC')
 */
export function withOpacity(hex: string, opacity: number): string {
  const alpha = Math.round(Math.min(1, Math.max(0, opacity)) * 255)
    .toString(16)
    .padStart(2, '0')
    .toUpperCase();
  return `${hex}${alpha}`;
}

/**
 * Reset all color assignments. Useful for testing or when context changes.
 */
export function resetColorAssignments(): void {
  colorAssignments.clear();
  usedIndices.clear();
}

/**
 * Darken a hex color by blending it toward black.
 * @param hex - Hex color string (e.g., '#5470c6')
 * @param amount - Blend amount between 0 (no change) and 1 (pure black)
 */
export function darkenColor(hex: string, amount: number): string {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  const t = Math.min(1, Math.max(0, amount));
  const dr = Math.round(r * (1 - t));
  const dg = Math.round(g * (1 - t));
  const db = Math.round(b * (1 - t));
  return `#${dr.toString(16).padStart(2, '0')}${dg.toString(16).padStart(2, '0')}${db.toString(16).padStart(2, '0')}`;
}

export const BLACK = '#000000';
export const WHITE = '#ffffff';

/**
 * Maps a query plan operation type to its associated color string.
 * Colors are derived from the CVA variants in QueryPlanNode.tsx.
 * Returns CSS color values suitable for programmatic use (SVG, canvas, etc.).
 *
 * @param operationType - The operation type string (e.g., 'source', 'join', 'aggregate')
 * @returns A CSS color string (Tailwind color name mapped to its standard hex value)
 */
const OPERATION_TYPE_COLORS: Record<string, string> = {
  source: '#3b82f6', // blue-500
  scan: '#3b82f6', // blue-500
  filesystemscan: '#3b82f6', // blue-500
  join: '#a855f7', // purple-500
  joinlocal: '#a855f7', // purple-500
  joinpartition: '#a855f7', // purple-500
  aggregate: '#22c55e', // green-500
  exchange: '#f97316', // orange-500
  output: '#ef4444', // red-500
  stage: '#4f46e5', // indigo-600
  local: '#f59e0b', // amber-500
  project: '#14b8a6', // teal-500
  filter: '#06b6d4', // cyan-500
  sort: '#8b5cf6', // violet-500
  limit: '#ec4899', // pink-500
  union: '#10b981', // emerald-500
  other: '#6b7280', // gray-500
};

export function getOperationTypeColor(operationType: string): string {
  return OPERATION_TYPE_COLORS[operationType.toLowerCase()] ?? OPERATION_TYPE_COLORS.other;
}

/**
 * Create a capacity->color resolver for timeline capacity series.
 * Multiple capacities use ordered palette assignment; a single capacity uses
 * key-based deterministic coloring to stay stable across timelines.
 */
export function createCapacitiesColorFn(
  capacityKeys: string[]
): (capacityName: string) => ChartColor {
  const colorMap =
    capacityKeys.length > 1
      ? assignColors(capacityKeys)
      : Object.fromEntries(capacityKeys.map(capacity => [capacity, getColorForKey(capacity)]));

  return (capacityName: string) => colorMap[capacityName] ?? getColorForKey(capacityName);
}

export function createFsmTypeColorFn(fsmTypes: { [key in string]?: FsmTypeDecl }): (
  stateName: string
) => ChartColor {
  const stateIndexMap = buildFsmStateIndexMap(fsmTypes);
  return (stateName: string) => {
    const stateIndex = stateIndexMap.get(stateName);
    return stateIndex != null ? getColorByIndex(stateIndex) : getColorForKey(stateName);
  };
}

function buildFsmStateIndexMap(fsmTypes?: { [key in string]?: FsmTypeDecl }): Map<string, number> {
  const stateIndexMap = new Map<string, number>();
  if (!fsmTypes) return stateIndexMap;

  for (const decl of Object.values(fsmTypes)) {
    if (!decl) continue;
    for (let i = 0; i < decl.states.length; i++) {
      stateIndexMap.set(decl.states[i]!.name, i);
    }
  }

  return stateIndexMap;
}

// ---------------------------------------------------------------------------
// Continuous color palettes (heatmap-style)
// ---------------------------------------------------------------------------

export const CONTINUOUS_PALETTES = {
  blue: { label: 'Blue' },
  teal: { label: 'Teal' },
  purple: { label: 'Purple' },
  orange: { label: 'Orange' },
  viridis: { label: 'Viridis' },
} as const;

export type ContinuousPaletteName = keyof typeof CONTINUOUS_PALETTES;

const VIRIDIS_STOPS: [number, number, number][] = [
  [68, 1, 84],
  [59, 82, 139],
  [33, 145, 140],
  [94, 201, 98],
  [253, 231, 37],
];

const NEUTRAL: [number, number, number] = [229, 231, 235];
const NEUTRAL_DARK: [number, number, number] = [55, 65, 81];

function blendToColor(
  r: number,
  g: number,
  b: number,
  t: number,
  neutral: [number, number, number] = NEUTRAL
): string {
  const c = Math.min(1, Math.max(0, t));
  const rr = Math.round(neutral[0] + (r - neutral[0]) * c);
  const gg = Math.round(neutral[1] + (g - neutral[1]) * c);
  const bb = Math.round(neutral[2] + (b - neutral[2]) * c);
  return `#${rr.toString(16).padStart(2, '0')}${gg.toString(16).padStart(2, '0')}${bb.toString(16).padStart(2, '0')}`;
}

/**
 * Compute a continuous color for a normalized value t ∈ [0, 1] using the given palette.
 */
export function continuousColor(t: number, palette: ContinuousPaletteName, darkMode = false): string {
  const neutral = darkMode ? NEUTRAL_DARK : NEUTRAL;
  switch (palette) {
    case 'blue':
      return blendToColor(59, 130, 246, t, neutral);
    case 'teal':
      return blendToColor(20, 184, 166, t, neutral);
    case 'purple':
      return blendToColor(168, 85, 247, t, neutral);
    case 'orange':
      return blendToColor(249, 115, 22, t, neutral);
    case 'viridis': {
      const clamped = Math.min(1, Math.max(0, t));
      const scaled = clamped * (VIRIDIS_STOPS.length - 1);
      const lo = Math.floor(scaled);
      const hi = Math.min(VIRIDIS_STOPS.length - 1, lo + 1);
      const frac = scaled - lo;
      const [r1, g1, b1] = VIRIDIS_STOPS[lo];
      const [r2, g2, b2] = VIRIDIS_STOPS[hi];
      const r = Math.round(r1 + (r2 - r1) * frac);
      const g = Math.round(g1 + (g2 - g1) * frac);
      const b = Math.round(b1 + (b2 - b1) * frac);
      return `#${r.toString(16).padStart(2, '0')}${g.toString(16).padStart(2, '0')}${b.toString(16).padStart(2, '0')}`;
    }
  }
}

/**
 * Returns the CSS gradient color stops for a palette legend bar.
 */
export function getLegendGradientStops(palette: ContinuousPaletteName, darkMode = false): string[] {
  if (palette === 'viridis') {
    return VIRIDIS_STOPS.map((_, i) =>
      continuousColor(i / (VIRIDIS_STOPS.length - 1), 'viridis', darkMode)
    );
  }
  return [continuousColor(0, palette, darkMode), continuousColor(1, palette, darkMode)];
}
