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
} from '../atoms/timeline';
import type { ZoomRange, SingleTimelineResponse } from '@quent/utils';

// Record-based replacement for atomFamily(timelineDataAtom(key))
export function useTimelineData(key: string): SingleTimelineResponse | undefined {
  const map = useAtomValue(timelineDataMapAtom);
  return map[key];
}

export const useZoomRange = () => useAtomValue(zoomRangeAtom);
export const useSetZoomRange = () => useSetAtom(zoomRangeAtom);
export const useDebouncedZoomRange = () => useAtomValue(debouncedZoomRangeAtom);
export const useSetDebouncedZoomRange = () => useSetAtom(debouncedZoomRangeAtom);
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
