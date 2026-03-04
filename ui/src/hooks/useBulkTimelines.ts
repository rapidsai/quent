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
  setOperatorOnEntries,
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
import { useBulkTimelineFetch } from './useBulkTimelineFetch';

const ZOOM_DEBOUNCE_MS = 150;

// useBulkTimelines — manages bulk fetching via Jotai atoms + TanStack Query
export function useBulkTimelines({
  engineId,
  queryId,
  rootItem,
  expandedIds,
  selectedTypes,
  entities,
}: {
  engineId: string;
  queryId: string;
  rootItem: TreeTableItem;
  expandedIds: Set<string>;
  selectedTypes: Map<string, string>;
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
  const windowSeconds = debouncedZoomRange.end - debouncedZoomRange.start;

  const baseVisibleEntries = useMemo(
    () =>
      collectVisibleEntries([rootItem], expandedIds, selectedTypes, entities, null, windowSeconds),
    [rootItem, expandedIds, selectedTypes, entities, windowSeconds]
  );
  useEffect(() => {
    store.set(visibleEntriesAtom, baseVisibleEntries);
  }, [baseVisibleEntries, store]);

  // Base bulk fetch (unfiltered, operator_id: null)
  const baseBulkData = useBulkTimelineFetch({
    engineId,
    queryId,
    debouncedZoomRange,
    entries: baseVisibleEntries,
  });

  // Operator bulk fetch (filtered, only when an operator is selected)
  const operatorBulkData = useBulkTimelineFetch({
    engineId,
    queryId,
    debouncedZoomRange,
    entries: baseVisibleEntries,
    operatorId,
    enabled: operatorId != null,
  });

  /* Once our base data is loaded and operator data if we have an operator id set
   * we can make the bulk data initialized true (allows single timelines to fetch themselves)
   */
  useEffect(() => {
    if (baseBulkData && (operatorId != null ? operatorBulkData : true)) {
      store.set(bulkInitializedAtom, true);
    }
  }, [baseBulkData, operatorId, operatorBulkData, store]);

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

      const newBaseEntries: Record<string, TimelineRequest<TaskFilter>> = {};
      for (const child of item.children) {
        const params = buildBulkParamsForItem(child, selectedTypes, entities, null, windowSeconds);
        const resourceTypeName = getResourceTypeName(params);
        const key = timelineCacheKey(child.id, resourceTypeName);
        if (!store.get(timelineDataAtom(key))) {
          newBaseEntries[child.id] = params;
        }
      }

      if (Object.keys(newBaseEntries).length === 0) return;

      const zoom = store.get(debouncedZoomRangeAtom);
      const windowSec = zoom.end - zoom.start;

      const bulkConfig = {
        num_bins: getAdaptiveNumBins(windowSec),
        start: zoom.start,
        end: zoom.end,
      };

      try {
        const baseRequest = queryClient.fetchQuery({
          queryKey: ['bulkTimelines', engineId, queryId, zoom, null, newBaseEntries],
          queryFn: () =>
            fetchBulkTimelines(engineId, {
              config: bulkConfig,
              entries: newBaseEntries,
              app_params: { query_id: queryId },
            }),
          staleTime: DEFAULT_STALE_TIME,
        });

        const operatorRequest = operatorId
          ? queryClient.fetchQuery({
              queryKey: ['bulkTimelines', engineId, queryId, zoom, operatorId, newBaseEntries],
              queryFn: () =>
                fetchBulkTimelines(engineId, {
                  config: bulkConfig,
                  entries: setOperatorOnEntries(newBaseEntries, operatorId),
                  app_params: { query_id: queryId },
                }),
              staleTime: DEFAULT_STALE_TIME,
            })
          : null;

        const [baseResponse, operatorResponse] = await Promise.all([baseRequest, operatorRequest]);

        for (const [id, entry] of Object.entries(baseResponse.entries)) {
          if (entry?.status === 'ok') {
            const resourceTypeName = getResourceTypeName(newBaseEntries[id]);
            const key = timelineCacheKey(id, resourceTypeName);
            store.set(timelineDataAtom(key), { data: entry.data, config: baseResponse.config });
          }
        }

        if (operatorResponse && operatorId) {
          for (const [id, entry] of Object.entries(operatorResponse.entries)) {
            if (entry?.status === 'ok') {
              const resourceTypeName = getResourceTypeName(newBaseEntries[id]);
              const key = timelineCacheKey(id, resourceTypeName, operatorId);
              store.set(timelineDataAtom(key), {
                data: entry.data,
                config: operatorResponse.config,
              });
            }
          }
        }
      } catch {
        // Individual ResourceTimeline components will fall back to self-fetch
      }
    },
    [
      rootItem,
      store,
      selectedTypes,
      entities,
      windowSeconds,
      queryClient,
      engineId,
      queryId,
      operatorId,
    ]
  );

  return { handleZoomChange, handleExpand } as const;
}
