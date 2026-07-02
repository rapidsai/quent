// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { atom } from 'jotai';
import type { ZoomRange, SingleTimelineResponse, TimelineRequest, OperatorFilter } from '@quent/utils';

/**
 * All dimensions that distinguish a cached timeline entry.
 *
 * Extensibility: add an optional field here and include it in the join inside
 * `timelineCacheKey`. Every call site passes a plain object, so the function
 * signature never changes.
 */
export interface TimelineCacheParams {
  resourceId: string;
  resourceTypeName: string;
  operatorId?: string | null;
  fsmTypeName?: string | null;
}

/** Build a composite cache key for per-item timeline data */
export function timelineCacheKey(params: TimelineCacheParams): string {
  return [
    params.resourceId,
    params.resourceTypeName,
    params.operatorId ?? '',
    params.fsmTypeName ?? '',
  ].join('|');
}

/** Per-item timeline data keyed by `timelineCacheKey(...)` — record-based, replaces atomFamily */
export const timelineDataMapAtom = atom<Record<string, SingleTimelineResponse>>({});

/** Immediate zoom range — updated on every zoom gesture */
export const zoomRangeAtom = atom<ZoomRange>({ start: 0, end: 0 });

/** Debounced zoom range — settles after ZOOM_DEBOUNCE_MS, drives the bulk query */
export const debouncedZoomRangeAtom = atom<ZoomRange>({ start: 0, end: 0 });

/**
 * Pointer-level hover state used to drive an app-rendered timeline tooltip.
 *
 * Written by the chart's own pointermove handler (see `Timeline.tsx`) and
 * carries enough information for an out-of-chart portal to render a tooltip:
 * the snapped bin index for series lookup, viewport coords for placement,
 * and a stable `sourceId` that lets exactly one Timeline take ownership of
 * the active hover (single-active invariant).
 */
export interface TimelineHoverState {
  /** Bin index under the pointer, already snapped to the nearest bin. */
  dataIndex: number;
  /**
   * Raw (un-snapped) x-axis value from `convertFromPixel`, in ms. Carried
   * alongside `dataIndex` so consumers can validate the snap or display the
   * exact pointer time independently of the snapped bin.
   */
  timestampMs: number;
  /** Pointer viewport coords for portal placement. */
  clientX: number;
  clientY: number;
  /** Stable id of the Timeline that owns the hover. */
  sourceId: string;
}

export const timelineHoverAtom = atom<TimelineHoverState | null>(null);

/** Start time in milliseconds — set once per query, never changes */
export const startTimeMsAtom = atom(0);

/** Flips to true after the first bulk fetch completes — gates individual fallback queries */
export const bulkInitializedAtom = atom(false);

/** Visible entries for bulk fetch — set in useEffect, read imperatively via store.get() */
export const visibleEntriesAtom = atom<Record<string, TimelineRequest<OperatorFilter>>>({});

/** When true, hides task annotation marks on timeline charts */
export const hideTasksAtom = atom(false);
