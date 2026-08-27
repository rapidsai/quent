// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { ZoomRange } from '@quent/utils';

export interface NormalizedZoomRange {
  range: ZoomRange;
  wasAdjusted: boolean;
}

export function normalizeZoomRange(
  range: ZoomRange,
  durationSeconds: number
): NormalizedZoomRange | null {
  if (!Number.isFinite(durationSeconds) || durationSeconds <= 0) {
    return null;
  }

  const start = Math.min(Math.max(range.start, 0), durationSeconds);
  const end = Math.min(Math.max(range.end, 0), durationSeconds);
  if (end <= start) {
    return {
      range: { start: 0, end: durationSeconds },
      wasAdjusted: true,
    };
  }

  return {
    range: { start, end },
    wasAdjusted: start !== range.start || end !== range.end,
  };
}

export function resolveCapturedZoomRange(
  currentRange: ZoomRange,
  durationSeconds: number
): ZoomRange | null {
  const currentIsValid =
    Number.isFinite(currentRange.start) &&
    Number.isFinite(currentRange.end) &&
    currentRange.start >= 0 &&
    currentRange.end > currentRange.start;

  return (
    normalizeZoomRange(
      currentIsValid ? currentRange : { start: 0, end: durationSeconds },
      durationSeconds
    )?.range ?? null
  );
}
