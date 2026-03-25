// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

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

/**
 * Lighten a hex color by blending it toward white.
 * @param hex - Hex color string (e.g., '#5470c6')
 * @param amount - Blend amount between 0 (no change) and 1 (pure white)
 */
export function lightenColor(hex: string, amount: number): string {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  const t = Math.min(1, Math.max(0, amount));
  const lr = Math.round(r + (255 - r) * t);
  const lg = Math.round(g + (255 - g) * t);
  const lb = Math.round(b + (255 - b) * t);
  return `#${lr.toString(16).padStart(2, '0')}${lg.toString(16).padStart(2, '0')}${lb.toString(16).padStart(2, '0')}`;
}

/**
 * Create a diagonal stripe canvas pattern for use in ECharts areaStyle/itemStyle.
 * Alternates between the given color and a darkened variant at -45deg.
 * @param hex - Base hex color
 * @param stripeWidth - Width of each stripe band in px (default 4)
 */
export function createStripePattern(hex: string, stripeWidth = 3): HTMLCanvasElement {
  const dark = darkenColor(hex, 0.2);
  const size = stripeWidth * 2;
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext('2d')!;

  ctx.fillStyle = hex;
  ctx.fillRect(0, 0, size, size);

  ctx.fillStyle = dark;
  ctx.beginPath();
  ctx.moveTo(0, size);
  ctx.lineTo(size, 0);
  ctx.lineTo(size, stripeWidth);
  ctx.lineTo(stripeWidth, size);
  ctx.closePath();
  ctx.fill();

  ctx.beginPath();
  ctx.moveTo(0, 0);
  ctx.lineTo(stripeWidth, 0);
  ctx.lineTo(0, stripeWidth);
  ctx.closePath();
  ctx.fill();

  return canvas;
}

/**
 * Create a dotted canvas pattern for use in ECharts areaStyle/itemStyle.
 * Draws evenly spaced dots of a darkened color on the base color.
 * @param hex - Base hex color
 * @param dotRadius - Radius of each dot in px (default 1.5)
 * @param spacing - Distance between dot centers in px (default 6)
 */
export function createDotPattern(hex: string, dotRadius = 1, spacing = 3): HTMLCanvasElement {
  const dark = darkenColor(hex, 0.3);
  const canvas = document.createElement('canvas');
  canvas.width = spacing;
  canvas.height = spacing;
  const ctx = canvas.getContext('2d')!;

  ctx.fillStyle = hex;
  ctx.fillRect(0, 0, spacing, spacing);

  ctx.fillStyle = dark;
  ctx.beginPath();
  ctx.arc(spacing / 2, spacing / 2, dotRadius, 0, Math.PI * 2);
  ctx.fill();

  return canvas;
}

/**
 * Create a crosshatch canvas pattern for use in ECharts areaStyle/itemStyle.
 * Draws thin diagonal lines in both directions over the base color.
 * @param hex - Base hex color
 * @param lineWidth - Width of each hatch line in px (default 1)
 * @param spacing - Distance between lines in px (default 6)
 */
export function createCrosshatchPattern(
  hex: string,
  lineWidth = 2,
  spacing = 6
): HTMLCanvasElement {
  const dark = darkenColor(hex, 0.25);
  const canvas = document.createElement('canvas');
  canvas.width = spacing;
  canvas.height = spacing;
  const ctx = canvas.getContext('2d')!;

  ctx.fillStyle = hex;
  ctx.fillRect(0, 0, spacing, spacing);

  ctx.strokeStyle = dark;
  ctx.lineWidth = lineWidth;

  // Top-left to bottom-right
  ctx.beginPath();
  ctx.moveTo(0, 0);
  ctx.lineTo(spacing, spacing);
  ctx.stroke();

  // Bottom-left to top-right
  ctx.beginPath();
  ctx.moveTo(0, spacing);
  ctx.lineTo(spacing, 0);
  ctx.stroke();

  return canvas;
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
