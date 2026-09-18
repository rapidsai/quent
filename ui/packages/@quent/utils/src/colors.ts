// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

/**
 * Centralized color palette and mapping utilities for charts and visualizations.
 */

export type PaletteTheme = 'light' | 'dark';
export type ColorPalette = readonly string[];

export const COLOR_PALETTES = {
  deterministic: [
    '#3b82f6',
    '#a855f7',
    '#22c55e',
    '#f97316',
    '#ef4444',
    '#4f46e5',
    '#f59e0b',
    '#14b8a6',
    '#06b6d4',
    '#8b5cf6',
    '#ec4899',
    '#10b981',
  ],
  timeline: {
    light: [
      '#44AA99',
      '#CC6677',
      '#332288',
      '#DDCC77',
      '#AA4499',
      '#88CCEE',
      '#882255',
      '#88AA55',
      '#666666',
    ],
    dark: [
      '#3D9485',
      '#B85858',
      '#4A68AA',
      '#B8A85E',
      '#9466BB',
      '#6BA8C8',
      '#B87A44',
      '#6E8C44',
      '#808080',
    ],
  },
} as const;

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
  if (used.size >= paletteSize) {
    return hashIndex;
  }
  let index = hashIndex;
  while (used.has(index)) {
    index = (index + 1) % paletteSize;
  }
  return index;
}

function requireColorPalette(palette: ColorPalette): void {
  if (palette.length === 0) {
    throw new Error('Color palettes must contain at least one color');
  }
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

export type DeterministicColorKey = string | number | bigint;

export interface DeterministicColorResolver {
  (value: DeterministicColorKey): string;
  <T>(value: T, keyOf: (value: T) => DeterministicColorKey): string;
}

export function normalizeDeterministicColorKey(value: DeterministicColorKey): string {
  return String(value).trim().toLowerCase();
}

export function getDeterministicColorFromPalette(
  value: DeterministicColorKey,
  palette: ColorPalette
): string {
  requireColorPalette(palette);
  const key = normalizeDeterministicColorKey(value);
  const index = hashString(key) % palette.length;
  return palette[index]!;
}

export function getDeterministicColor(value: DeterministicColorKey): string {
  return getDeterministicColorFromPalette(value, COLOR_PALETTES.deterministic);
}

export function buildDeterministicColorMap(
  values: Iterable<DeterministicColorKey>,
  palette?: ColorPalette
): Map<string, string>;
export function buildDeterministicColorMap<T>(
  values: Iterable<T>,
  keyOf: (value: T) => DeterministicColorKey,
  palette?: ColorPalette
): Map<string, string>;
export function buildDeterministicColorMap<T>(
  values: Iterable<T>,
  keyOfOrPalette?: ((value: T) => DeterministicColorKey) | ColorPalette,
  explicitPalette?: ColorPalette
): Map<string, string> {
  const keyOf = typeof keyOfOrPalette === 'function' ? keyOfOrPalette : undefined;
  const palette =
    (typeof keyOfOrPalette === 'function' ? explicitPalette : keyOfOrPalette) ??
    COLOR_PALETTES.deterministic;
  requireColorPalette(palette);
  const sorted = [
    ...new Set(
      [...values].map(value =>
        normalizeDeterministicColorKey(keyOf ? keyOf(value) : (value as DeterministicColorKey))
      )
    ),
  ].sort();
  const used = new Set<number>();
  const map = new Map<string, string>();
  for (const key of sorted) {
    const index = pickPaletteIndex(key, palette.length, used);
    used.add(index);
    map.set(key, palette[index]!);
  }
  return map;
}

export function extendDeterministicColorMap(
  colorMap: ReadonlyMap<string, string>,
  values: Iterable<DeterministicColorKey>,
  palette: ColorPalette = COLOR_PALETTES.deterministic
): Map<string, string> {
  requireColorPalette(palette);
  const map = new Map(colorMap);
  const used = new Set(
    [...colorMap.values()].map(color => palette.indexOf(color)).filter(index => index >= 0)
  );
  const newKeys = [
    ...new Set([...values].map(value => normalizeDeterministicColorKey(value))),
  ].sort();

  for (const key of newKeys) {
    if (map.has(key)) {
      continue;
    }
    const index = pickPaletteIndex(key, palette.length, used);
    used.add(index);
    map.set(key, palette[index]!);
  }
  return map;
}

export function createDeterministicColorResolver(
  colorMap: ReadonlyMap<string, string>,
  palette: ColorPalette = COLOR_PALETTES.deterministic
): DeterministicColorResolver {
  return ((value: DeterministicColorKey, keyOf?: (value: unknown) => DeterministicColorKey) => {
    const key = normalizeDeterministicColorKey(keyOf ? keyOf(value) : value);
    return colorMap.get(key) ?? getDeterministicColorFromPalette(key, palette);
  }) as DeterministicColorResolver;
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
