import { atom } from 'jotai';
import { atomFamily } from 'jotai-family';
import type { SingleTimelineResponse } from '~quent/types/SingleTimelineResponse';
import type { TimelineRequest } from '~quent/types/TimelineRequest';
import type { TaskFilter } from '~quent/types/TaskFilter';
import type { ZoomRange } from '@/components/timeline/TimelineController';
import type { XAxisRange } from '@/components/timeline/Timeline';

/** Build a composite cache key for per-item timeline data */
export function timelineCacheKey(
  resourceId: string,
  resourceTypeName: string,
  operatorId: string | null = null
): string {
  return `${resourceId}|${resourceTypeName}|${operatorId ?? ''}`;
}

/** Per-item timeline data keyed by `timelineCacheKey(resourceId, resourceTypeName, operatorId)` */
export const timelineDataAtom = atomFamily(() =>
  atom<SingleTimelineResponse | undefined>(undefined)
);

/** Immediate zoom range — updated on every zoom gesture */
export const zoomRangeAtom = atom<ZoomRange>({ start: 0, end: 0 });

/** Debounced zoom range — settles after ZOOM_DEBOUNCE_MS, drives the bulk query */
export const debouncedZoomRangeAtom = atom<ZoomRange>({ start: 0, end: 0 });

/** Which timeline row is currently hovered (for tooltip display) */
export const hoveredTimelineIdAtom = atom<string | null>(null);

/**
 * Derived per-item hover check — only the two rows involved in a hover
 * change (old and new) re-render, not all rows.
 */
export const isTimelineHoveredAtom = atomFamily((itemId: string) =>
  atom(get => get(hoveredTimelineIdAtom) === itemId)
);

/** Start time in milliseconds — set once per query, never changes */
export const startTimeMsAtom = atom(0);

/** Derived xAxis bounds from zoom range + start time */
export const xAxisRangeAtom = atom<XAxisRange>(get => {
  const startTimeMs = get(startTimeMsAtom);
  const zoomRange = get(zoomRangeAtom);
  return {
    min: startTimeMs + zoomRange.start * 1_000,
    max: startTimeMs + zoomRange.end * 1_000,
  };
});

/** Flips to true after the first bulk fetch completes — gates individual fallback queries */
export const bulkInitializedAtom = atom(false);

/** Visible entries for bulk fetch — set in useEffect, read imperatively via store.get() */
export const visibleEntriesAtom = atom<Record<string, TimelineRequest<TaskFilter>>>({});
