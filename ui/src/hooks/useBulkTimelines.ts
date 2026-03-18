import { useCallback, useEffect, useMemo, useRef } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useAtomValue, useStore } from 'jotai';
import { fetchBulkTimelines, DEFAULT_STALE_TIME } from '@/services/api';
import type { QueryEntities } from '~quent/types/QueryEntities';
import type { TimelineRequest } from '~quent/types/TimelineRequest';
import type { TaskFilter } from '~quent/types/TaskFilter';
import type { ZoomRange } from '@/components/timeline/TimelineController';
import { TreeTableItem } from '@/components/resource-tree/types';
import {
  findItemById,
  buildBulkParamsForItem,
  collectVisibleEntries,
  getAdaptiveNumBins,
  getResourceTypeName,
  getFsmTypeName,
} from '@/lib/timeline.utils';
import {
  timelineCacheKey,
  timelineDataAtom,
  zoomRangeAtom,
  debouncedZoomRangeAtom,
  bulkInitializedAtom,
  visibleEntriesAtom,
} from '@/atoms/timeline';
import { selectedNodeIdsAtom } from '@/atoms/dag';
import {
  useBulkTimelineFetch,
  buildMergedBulkEntries,
  applyBulkTimelineResponse,
} from './useBulkTimelineFetch';

const ZOOM_DEBOUNCE_MS = 150;

// useBulkTimelines — manages bulk fetching via Jotai atoms + TanStack Query
export function useBulkTimelines({
  engineId,
  queryId,
  rootItem,
  expandedIds,
  selectedTypes,
  groupFsmFilters,
  entities,
}: {
  engineId: string;
  queryId: string;
  rootItem: TreeTableItem;
  expandedIds: Set<string>;
  selectedTypes: Map<string, string>;
  groupFsmFilters?: Map<string, string | null>;
  entities: QueryEntities;
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
      num_bins: getAdaptiveNumBins(),
      start: debouncedZoomRange.start,
      end: debouncedZoomRange.end,
    }),
    [debouncedZoomRange]
  );

  const baseVisibleEntries = useMemo(
    () =>
      collectVisibleEntries(
        [rootItem],
        expandedIds,
        selectedTypes,
        entities,
        bulkConfig,
        groupFsmFilters
      ),
    [rootItem, expandedIds, selectedTypes, entities, bulkConfig, groupFsmFilters]
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

      const item = findItemById(rootItem, itemId);
      if (!item?.children) return;

      const zoom = store.get(debouncedZoomRangeAtom);
      const expandConfig = {
        num_bins: getAdaptiveNumBins(),
        start: zoom.start,
        end: zoom.end,
      };

      const newBaseEntries: Record<string, TimelineRequest<TaskFilter>> = {};
      for (const child of item.children) {
        const params = buildBulkParamsForItem(
          child,
          selectedTypes,
          entities,
          expandConfig,
          groupFsmFilters
        );
        const resourceTypeName = getResourceTypeName(params);
        const fsmTypeName = getFsmTypeName(params);
        const key = timelineCacheKey({ resourceId: child.id, resourceTypeName, fsmTypeName });
        if (!store.get(timelineDataAtom(key))) {
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
    ]
  );

  return { handleZoomChange, handleExpand } as const;
}
