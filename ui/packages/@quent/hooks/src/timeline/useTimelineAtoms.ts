// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useId, useMemo } from 'react';
import { useAtomValue, useSetAtom, useStore } from 'jotai';
import { useHydrateAtoms } from 'jotai/utils';
import {
  timelineDataMapAtom,
  zoomRangeAtom,
  debouncedZoomRangeAtom,
  timelineHoverAtom,
  timelinePointerAtom,
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
import { isTimelineUtilizationAllZero } from './timeline.utils';

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
  if (!request) {
    return { data: undefined, isStale: false };
  }
  const key = timelineCacheKey({
    resourceId,
    resourceTypeName: getResourceTypeName(request),
    fsmTypeName: getFsmTypeName(request),
  });
  const data = timelineDataMap[key];
  if (!data) {
    return { data: undefined, isStale: false };
  }
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

/**
 * Resource ids whose binned utilization is entirely zero across the current
 * (non-stale) zoom window. Zero utilization means no usages occurred either,
 * so callers use this to hide UI that only makes sense when usages exist
 * (e.g. the per-resource long-entities lane).
 */
export function useZeroUtilizationResourceIds(): ReadonlySet<string> {
  const timelineDataMap = useAtomValue(timelineDataMapAtom);
  const visibleEntries = useAtomValue(visibleEntriesAtom);
  const activeSpan = useAtomValue(debouncedZoomRangeAtom);

  return useMemo(() => {
    const zeroResourceIds = new Set<string>();
    for (const [resourceId, request] of Object.entries(visibleEntries)) {
      const key = timelineCacheKey({
        resourceId,
        resourceTypeName: getResourceTypeName(request),
        fsmTypeName: getFsmTypeName(request),
      });
      const data = timelineDataMap[key];
      if (!data) {
        continue;
      }
      const tolerance = data.config.bin_duration;
      const matchesActiveSpan =
        Math.abs(data.config.span.start - activeSpan.start) <= tolerance &&
        Math.abs(data.config.span.end - activeSpan.end) <= tolerance;
      if (matchesActiveSpan && isTimelineUtilizationAllZero(data.data)) {
        zeroResourceIds.add(resourceId);
      }
    }
    return zeroResourceIds;
  }, [timelineDataMap, visibleEntries, activeSpan]);
}

export const useZoomRange = () => useAtomValue(zoomRangeAtom);
export const useGetZoomRange = () => {
  const store = useStore();
  return useCallback(() => store.get(zoomRangeAtom), [store]);
};
export const useSetZoomRange = () => useSetAtom(zoomRangeAtom);
export function useReadZoomRange() {
  const store = useStore();
  return useCallback(() => store.get(zoomRangeAtom), [store]);
}
export const useDebouncedZoomRange = () => useAtomValue(debouncedZoomRangeAtom);
export const useSetDebouncedZoomRange = () => useSetAtom(debouncedZoomRangeAtom);
export const useLongEntityDensity = () => useAtomValue(longEntityDensityAtom);
export const useSetLongEntityDensity = () => useSetAtom(longEntityDensityAtom);
export const useTimelineHover = () => useAtomValue(timelineHoverAtom);
export const useSetTimelineHover = () => useSetAtom(timelineHoverAtom);
export const useTimelinePointerRatio = () => useAtomValue(timelinePointerAtom)?.ratio ?? null;
export function useTimelinePointerPublisher() {
  const ownerId = useId();
  const store = useStore();
  const setPointer = useSetAtom(timelinePointerAtom);
  const publish = useCallback(
    (ratio: number) => {
      setPointer({ ratio: Math.min(1, Math.max(0, ratio)), ownerId });
    },
    [ownerId, setPointer]
  );
  const clear = useCallback(() => {
    const ownedPointer = store.get(timelinePointerAtom);
    if (ownedPointer?.ownerId !== ownerId) {
      return;
    }
    const clearIfUnchanged = () => {
      if (store.get(timelinePointerAtom) === ownedPointer) {
        setPointer(null);
      }
    };
    if (typeof requestAnimationFrame === 'function') {
      requestAnimationFrame(clearIfUnchanged);
    } else {
      setTimeout(clearIfUnchanged, 0);
    }
  }, [ownerId, setPointer, store]);

  useEffect(
    () => () => {
      if (store.get(timelinePointerAtom)?.ownerId === ownerId) {
        setPointer(null);
      }
    },
    [ownerId, setPointer, store]
  );

  return { publish, clear };
}
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
