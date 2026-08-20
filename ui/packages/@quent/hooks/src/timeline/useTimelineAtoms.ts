// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useAtomValue, useSetAtom } from 'jotai';
import { useHydrateAtoms } from 'jotai/utils';
import {
  timelineDataMapAtom,
  zoomRangeAtom,
  debouncedZoomRangeAtom,
  timelineHoverAtom,
  startTimeMsAtom,
  bulkInitializedAtom,
  visibleEntriesAtom,
  longEntityDensityAtom,
  timelineCacheKey,
} from '../atoms/timeline';
import {
  getFsmTypeName,
  getResourceTypeName,
  type ZoomRange,
  type SingleTimelineResponse,
} from '@quent/utils';

// Record-based replacement for atomFamily(timelineDataAtom(key))
export function useTimelineData(key: string): SingleTimelineResponse | undefined {
  const map = useAtomValue(timelineDataMapAtom);
  return map[key];
}

function useReturnedTimelineState(resourceId: string): {
  data: SingleTimelineResponse | undefined;
  isStale: boolean;
} {
  const timelineDataMap = useAtomValue(timelineDataMapAtom);
  const visibleEntries = useAtomValue(visibleEntriesAtom);
  const activeSpan = useAtomValue(debouncedZoomRangeAtom);
  const request = visibleEntries[resourceId];
  if (!request) return { data: undefined, isStale: false };
  const key = timelineCacheKey({
    resourceId,
    resourceTypeName: getResourceTypeName(request),
    fsmTypeName: getFsmTypeName(request),
  });
  const data = timelineDataMap[key];
  if (!data) return { data: undefined, isStale: false };
  const tolerance = data.config.bin_duration;
  const matchesActiveSpan =
    Math.abs(data.config.span.start - activeSpan.start) <= tolerance &&
    Math.abs(data.config.span.end - activeSpan.end) <= tolerance;
  return matchesActiveSpan ? { data, isStale: false } : { data: undefined, isStale: true };
}

export function useReturnedTimelineNumBins(resourceId: string): number | undefined {
  const { data } = useReturnedTimelineState(resourceId);
  const numBins = Number(data?.config.num_bins);
  return Number.isInteger(numBins) && numBins > 0 ? numBins : undefined;
}

export function useReturnedTimelineIsStale(resourceId: string): boolean {
  return useReturnedTimelineState(resourceId).isStale;
}

export const useZoomRange = () => useAtomValue(zoomRangeAtom);
export const useSetZoomRange = () => useSetAtom(zoomRangeAtom);
export const useDebouncedZoomRange = () => useAtomValue(debouncedZoomRangeAtom);
export const useSetDebouncedZoomRange = () => useSetAtom(debouncedZoomRangeAtom);
export const useLongEntityDensity = () => useAtomValue(longEntityDensityAtom);
export const useSetLongEntityDensity = () => useSetAtom(longEntityDensityAtom);
export const useTimelineHover = () => useAtomValue(timelineHoverAtom);
export const useSetTimelineHover = () => useSetAtom(timelineHoverAtom);
export const useStartTimeMs = () => useAtomValue(startTimeMsAtom);
export const useSetStartTimeMs = () => useSetAtom(startTimeMsAtom);
export const useBulkInitialized = () => useAtomValue(bulkInitializedAtom);
export const useSetBulkInitialized = () => useSetAtom(bulkInitializedAtom);
export const useVisibleEntries = () => useAtomValue(visibleEntriesAtom);
export const useSetVisibleEntries = () => useSetAtom(visibleEntriesAtom);

/**
 * Hydrates the timeline atoms with initial values synchronously during render.
 * Use this in the root component of a query view to initialize zoom and start time
 * before child components read them.
 */
export function useHydrateTimelineAtoms(params: {
  zoomRange: ZoomRange;
  debouncedZoomRange: ZoomRange;
  startTimeMs: number;
}): void {
  useHydrateAtoms([
    [zoomRangeAtom, params.zoomRange],
    [debouncedZoomRangeAtom, params.debouncedZoomRange],
    [startTimeMsAtom, params.startTimeMs],
  ]);
}
