// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

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
  extended: {
    /** Qualitative palette — light mode */
    light: [
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
    /** Qualitative palette — dark mode (muted, lower contrast) */
    dark: [
      '#3D9485', // Teal
      '#B85858', // Coral Red
      '#4A68AA', // Steel Blue
      '#B8A85E', // Sand
      '#9466BB', // Violet
      '#6BA8C8', // Cyan
      '#B87A44', // Amber
      '#6E8C44', // Muted Lime
      '#808080', // Grey
    ],
  },
} as const;

export type PaletteName = keyof typeof PALETTES;
export type ChartColor = string;
export type PaletteTheme = 'light' | 'dark';
type PaletteValue = (typeof PALETTES)[PaletteName];

// Current active palette
let activePalette: PaletteName = 'extended';

function resolvePalette(palette: PaletteValue, theme: PaletteTheme = 'light'): readonly string[] {
  if ('light' in palette && 'dark' in palette) {
    return palette[theme];
  }
  return palette;
}

/**
 * Get the currently active palette.
 */
export function getActivePalette(theme: PaletteTheme = 'light'): readonly string[] {
  return resolvePalette(PALETTES[activePalette], theme);
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
export function getPalette(name: PaletteName, theme: PaletteTheme = 'light'): readonly string[] {
  return resolvePalette(PALETTES[name], theme);
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

/**
 * Pick a palette index for `key` using hash + linear probe.
 * Probes forward from the hash index, skipping anything already in `used`,
 * until it finds a free slot. If `used` is already full (size >= paletteSize)
 * the bare hash index is returned — duplicates are unavoidable past the
 * palette size, and the early return prevents an infinite probe loop.
 */
function pickPaletteIndex(key: string, paletteSize: number, used: Set<number>): number {
  const hashIndex = hashString(key) % paletteSize;
  if (used.size >= paletteSize) return hashIndex;
  let index = hashIndex;
  while (used.has(index)) index = (index + 1) % paletteSize;
  return index;
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
export function getColorForKey(key: string, theme: PaletteTheme): ChartColor {
  const palette = getActivePalette(theme);

  if (colorAssignments.has(key)) {
    return palette[colorAssignments.get(key)!];
  }

  const index = pickPaletteIndex(key, palette.length, usedIndices);
  colorAssignments.set(key, index);
  usedIndices.add(index);
  return palette[index];
}

/**
 * Assign colors to an array of keys in order.
 * Useful for batch assignment to maintain consistent ordering.
 */
export function assignColors<T extends string>(
  keys: T[],
  theme: PaletteTheme
): Record<T, ChartColor> {
  const palette = getActivePalette(theme);
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
  capacityKeys: string[],
  theme: PaletteTheme
): (capacityName: string) => ChartColor {
  const colorMap =
    capacityKeys.length > 1
      ? assignColors(capacityKeys, theme)
      : Object.fromEntries(
          capacityKeys.map(capacity => [capacity, getColorForKey(capacity, theme)])
        );

  return (capacityName: string) => colorMap[capacityName] ?? getColorForKey(capacityName, theme);
}

/**
 * Get a color by index from the active palette (wraps around).
 */
export function getColorByIndex(index: number, theme: PaletteTheme): ChartColor {
  const palette = getActivePalette(theme);
  return palette[index % palette.length];
}

export function createFsmTypeColorFn(
  fsmTypes: { [key in string]?: FsmTypeDecl },
  theme: PaletteTheme
): (stateName: string) => ChartColor {
  const stateIndexMap = buildFsmStateIndexMap(fsmTypes);
  return (stateName: string) => {
    const stateIndex = stateIndexMap.get(stateName);
    return stateIndex != null
      ? getColorByIndex(stateIndex, theme)
      : getColorForKey(stateName, theme);
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
 * Returns true if the given hex color (#rrggbb) has high perceived luminance,
 * meaning dark text should be used on top of it for readability.
 */
export function isLightColor(hex: string): boolean {
  const r = parseInt(hex.slice(1, 3), 16) / 255;
  const g = parseInt(hex.slice(3, 5), 16) / 255;
  const b = parseInt(hex.slice(5, 7), 16) / 255;
  return 0.299 * r + 0.587 * g + 0.114 * b > 0.5;
}

/**
 * Maps a query plan operation type to its associated color string.
 * Colors are derived from the CVA variants in QueryPlanNode.tsx.
 * Returns CSS color values suitable for programmatic use (SVG, canvas, etc.).
 *
 * @param operationType - The operation type string (e.g., 'source', 'join', 'aggregate')
 * @returns A CSS color string (Tailwind color name mapped to its standard hex value)
 */

const OPERATOR_PALETTE = [
  '#3b82f6', // blue-500
  '#a855f7', // purple-500
  '#22c55e', // green-500
  '#f97316', // orange-500
  '#ef4444', // red-500
  '#4f46e5', // indigo-600
  '#f59e0b', // amber-500
  '#14b8a6', // teal-500
  '#06b6d4', // cyan-500
  '#8b5cf6', // violet-500
  '#ec4899', // pink-500
  '#10b981', // emerald-500
];

export function getOperationTypeColor(operationType: string): string {
  const index = hashString(operationType.toLowerCase()) % OPERATOR_PALETTE.length;
  return OPERATOR_PALETTE[index]!;
}

export function buildOperatorColorMap(operatorTypes: string[]): Map<string, string> {
  const sorted = [...new Set(operatorTypes.map(t => t.toLowerCase()))].sort();
  const used = new Set<number>();
  const map = new Map<string, string>();
  for (const type of sorted) {
    const index = pickPaletteIndex(type, OPERATOR_PALETTE.length, used);
    used.add(index);
    map.set(type, OPERATOR_PALETTE[index]!);
  }
  return map;
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

const NEUTRAL: [number, number, number] = [255, 255, 255];
const NEUTRAL_DARK: [number, number, number] = [14, 22, 33];

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
export function continuousColor(
  t: number,
  palette: ContinuousPaletteName,
  darkMode = false
): string {
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
