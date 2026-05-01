// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useMemo, useRef } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useAtomValue, useStore } from 'jotai';
import { fetchBulkTimelines, DEFAULT_STALE_TIME } from '@quent/client';
import type { QueryEntities, TimelineRequest, TaskFilter, ZoomRange } from '@quent/utils';
import { getResourceTypeName, getFsmTypeName } from './timeline.utils';
import {
  timelineCacheKey,
  timelineDataMapAtom,
  zoomRangeAtom,
  debouncedZoomRangeAtom,
  bulkInitializedAtom,
  visibleEntriesAtom,
} from '../atoms/timeline';
import { selectedNodeIdsAtom } from '../atoms/dag';
import {
  useBulkTimelineFetch,
  buildMergedBulkEntries,
  applyBulkTimelineResponse,
} from './useBulkTimelineFetch';

/**
 * Minimal tree node interface.
 * App code can pass TreeTableItem directly — structural typing ensures compatibility.
 */
export interface TreeNode {
  id: string;
  children?: TreeNode[];
}

const ZOOM_DEBOUNCE_MS = 150;
const MAX_TIMELINE_BINS = 400;

/**
 * useBulkTimelines — manages bulk fetching via Jotai atoms + TanStack Query.
 *
 * App-layer utilities that depend on TreeTableItem are injected to avoid
 * coupling this package to the component layer.
 */
export function useBulkTimelines<T extends TreeNode>({
  engineId,
  queryId,
  rootItem,
  expandedIds,
  selectedTypes,
  groupFsmFilters,
  entities,
  collectVisibleEntriesFn,
  buildBulkParamsFn,
  findItemByIdFn,
}: {
  engineId: string;
  queryId: string;
  rootItem: T;
  expandedIds: Set<string>;
  selectedTypes: Map<string, string>;
  groupFsmFilters?: Map<string, string | null>;
  entities: QueryEntities;
  collectVisibleEntriesFn: (
    items: T[],
    expandedIds: Set<string>,
    selectedTypes: Map<string, string>,
    entities: QueryEntities,
    config: { num_bins: number; start: number; end: number },
    groupFsmFilters?: Map<string, string | null>
  ) => Record<string, TimelineRequest<TaskFilter>>;
  buildBulkParamsFn: (
    item: T,
    selectedTypes: Map<string, string>,
    entities: QueryEntities,
    config: { num_bins: number; start: number; end: number },
    groupFsmFilters?: Map<string, string | null>
  ) => TimelineRequest<TaskFilter>;
  findItemByIdFn: (root: T, id: string) => T | undefined;
}) {
  const store = useStore();
  const queryClient = useQueryClient();
  const debounceTimerRef = useRef<ReturnType<typeof setTimeout>>(null);
  const selectedNodeIds = useAtomValue(selectedNodeIdsAtom);
  const operatorId = selectedNodeIds.size > 0 ? selectedNodeIds.values().next().value! : null;

  useEffect(() => {
    return () => {
      if (debounceTimerRef.current) clearTimeout(debounceTimerRef.current);
    };
  }, []);

  const debouncedZoomRange = useAtomValue(debouncedZoomRangeAtom);
  const bulkConfig = useMemo(
    () => ({
      num_bins: MAX_TIMELINE_BINS,
      start: debouncedZoomRange.start,
      end: debouncedZoomRange.end,
    }),
    [debouncedZoomRange]
  );

  const baseVisibleEntries = useMemo(
    () =>
      collectVisibleEntriesFn(
        [rootItem],
        expandedIds,
        selectedTypes,
        entities,
        bulkConfig,
        groupFsmFilters
      ),

    [
      rootItem,
      expandedIds,
      selectedTypes,
      entities,
      bulkConfig,
      groupFsmFilters,
      collectVisibleEntriesFn,
    ]
  );

  useEffect(() => {
    store.set(visibleEntriesAtom, baseVisibleEntries);
  }, [baseVisibleEntries, store]);

  const bulkData = useBulkTimelineFetch({
    engineId,
    queryId,
    debouncedZoomRange,
    entries: baseVisibleEntries,
    operatorId,
  });

  useEffect(() => {
    if (bulkData) {
      store.set(bulkInitializedAtom, true);
    }
  }, [bulkData, store]);

  // Zoom change handler — stable, uses store imperatively
  const handleZoomChange = useCallback(
    (range: ZoomRange) => {
      store.set(zoomRangeAtom, range);

      if (debounceTimerRef.current) clearTimeout(debounceTimerRef.current);
      debounceTimerRef.current = setTimeout(() => {
        store.set(debouncedZoomRangeAtom, range);
        debounceTimerRef.current = null;
      }, ZOOM_DEBOUNCE_MS);
    },
    [store]
  );

  // Expand handler — fetches base + operator data for newly expanded children
  const handleExpand = useCallback(
    async (itemId: string, isExpanded: boolean) => {
      if (!isExpanded) return;

      const item = findItemByIdFn(rootItem, itemId);
      if (!item?.children) return;

      const zoom = store.get(debouncedZoomRangeAtom);
      const expandConfig = {
        num_bins: MAX_TIMELINE_BINS,
        start: zoom.start,
        end: zoom.end,
      };

      const newBaseEntries: Record<string, TimelineRequest<TaskFilter>> = {};
      for (const child of item.children as T[]) {
        const params = buildBulkParamsFn(
          child,
          selectedTypes,
          entities,
          expandConfig,
          groupFsmFilters
        );
        const resourceTypeName = getResourceTypeName(params);
        const fsmTypeName = getFsmTypeName(params);
        const key = timelineCacheKey({ resourceId: child.id, resourceTypeName, fsmTypeName });
        if (!store.get(timelineDataMapAtom)[key]) {
          newBaseEntries[child.id] = params;
        }
      }

      if (Object.keys(newBaseEntries).length === 0) return;

      const {
        entries: expandEntries,
        idToMeta: expandIdToMeta,
        requestKey: expandRequestKey,
      } = buildMergedBulkEntries(newBaseEntries, operatorId);

      try {
        const response = await queryClient.fetchQuery({
          queryKey: ['bulkTimelines', engineId, queryId, zoom, expandRequestKey],
          queryFn: () =>
            fetchBulkTimelines(engineId, {
              entries: expandEntries,
              app_params: { query_id: queryId },
            }),
          staleTime: DEFAULT_STALE_TIME,
        });

        applyBulkTimelineResponse(response, expandIdToMeta, store);
      } catch {
        // Individual ResourceTimeline components will fall back to self-fetch
      }
    },
    [
      rootItem,
      store,
      selectedTypes,
      groupFsmFilters,
      entities,
      queryClient,
      engineId,
      queryId,
      operatorId,
      buildBulkParamsFn,
      findItemByIdFn,
    ]
  );

  return { handleZoomChange, handleExpand } as const;
}
