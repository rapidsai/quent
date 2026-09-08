// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

export function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

export function parseOptionalNumber(value: string): number | null {
  if (value.trim() === '') {
    return null;
  }
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

/** Picks a "nice" step (1/2/5 * 10^n) so a slider spanning `span` units has roughly 100-200 stops. */
export function niceSliderStep(span: number): number {
  if (!Number.isFinite(span) || span <= 0) {
    return 0.1;
  }
  const rawStep = span / 150;
  const magnitude = 10 ** Math.floor(Math.log10(rawStep));
  const normalized = rawStep / magnitude;
  const niceNormalized = normalized < 1.5 ? 1 : normalized < 3.5 ? 2 : normalized < 7.5 ? 5 : 10;
  return niceNormalized * magnitude;
}

export function formatStep(value: number, step: number): string {
  const decimals = step > 0 && step < 1 ? Math.min(6, Math.ceil(-Math.log10(step))) : 0;
  return value.toFixed(decimals);
}

/**
 * Resolves a raw slider position to a value string. `step` is a "nice" round number picked for
 * a good number of visual stops, so it rarely divides `max - min` evenly — the highest/lowest
 * reachable step-grid point is then strictly inside `[min, max]`, making the true bound
 * undraggable. Snap to the exact bound whenever the raw position is within one step of it, so
 * dragging a thumb all the way to either end always resolves to the real min/max.
 */
export function resolveSliderValue(raw: number, step: number, min: number, max: number): string {
  if (max - raw < step) {
    return String(max);
  }
  if (raw - min < step) {
    return String(min);
  }
  return String(clamp(Number(formatStep(raw, step)), min, max));
}
