/**
 * Centralized color palette and mapping utilities for charts and visualizations.
 */

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
} as const;

export type PaletteName = keyof typeof PALETTES;
export type ChartColor = string;

// Current active palette
let activePalette: PaletteName = 'echarts';

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

// Track color assignments: key -> palette index
const colorAssignments = new Map<string, number>();
// Track which palette indices are in use
const usedIndices = new Set<number>();

/**
 * Get a deterministic color for a given key.
 * Uses a hash of the key to select a color, with collision handling to ensure
 * all colors are used before any color is assigned to multiple keys.
 */
export function getColorForKey(key: string): ChartColor {
  const palette = getActivePalette();

  // Return cached assignment if exists
  if (colorAssignments.has(key)) {
    return palette[colorAssignments.get(key)!];
  }

  // Get hash-based starting index
  const hashIndex = hashString(key) % palette.length;

  // If all colors are used, just use the hash index (allow duplicates)
  if (usedIndices.size >= palette.length) {
    colorAssignments.set(key, hashIndex);
    return palette[hashIndex];
  }

  // Find an available index, starting from hash index and probing forward
  let index = hashIndex;
  while (usedIndices.has(index)) {
    index = (index + 1) % palette.length;
  }

  // Assign and mark as used
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
 * Get a color by index from the active palette (wraps around).
 */
export function getColorByIndex(index: number): ChartColor {
  const palette = getActivePalette();
  return palette[index % palette.length];
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

export const BLACK = '#000000';
export const WHITE = '#ffffff';
