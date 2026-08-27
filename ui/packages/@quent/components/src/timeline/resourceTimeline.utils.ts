// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { SingleTimelineResponse } from '@quent/utils';

/** Overlay data retained for one canonical operator cache key. */
export interface RetainedOverlayData {
  cacheKey: string;
  data: SingleTimelineResponse;
}

/** Selects fresh or same-key retained overlay data. */
export function resolveOverlayData(
  currentData: SingleTimelineResponse | undefined,
  retainedData: RetainedOverlayData | null,
  currentCacheKey: string,
  hasOperatorFilter: boolean
): SingleTimelineResponse | undefined {
  if (!hasOperatorFilter) {
    return undefined;
  }
  return (
    currentData ?? (retainedData?.cacheKey === currentCacheKey ? retainedData.data : undefined)
  );
}
