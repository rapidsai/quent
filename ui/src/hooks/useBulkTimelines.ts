import { useState, useCallback, useEffect, useMemo, useRef } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useAtomValue, useStore } from 'jotai';
import { fetchBulkTimelines, DEFAULT_STALE_TIME } from '@/services/api';
import type { BulkTimelineRequestParams } from '~quent/types/BulkTimelineRequestParams';
import type { QueryEntities } from '~quent/types/QueryEntities';
import type { ZoomRange } from '@/components/timeline/TimelineController';
import { TreeTableItem } from '@/components/resource-tree/types';
import {
  bulkEntryToTimelineResponse,
  findItemById,
  buildBulkParamsForItem,
  collectVisibleEntries,
  getAdaptiveNumBins,
} from '@/lib/timeline.utils';
import {
  timelineDataAtom,
  zoomRangeAtom,
  debouncedZoomRangeAtom,
  bulkInitializedAtom,
  visibleEntriesAtom,
} from '@/atoms/timeline';

const ZOOM_DEBOUNCE_MS = 150;

// useExpandedIds — tracks which tree nodes are expanded
export function useExpandedIds(initialId?: string) {
  const [expandedIds, setExpandedIds] = useState<Set<string>>(() => {
    return initialId ? new Set([initialId]) : new Set();
  });

  const handleExpandChange = useCallback((itemId: string, isExpanded: boolean) => {
    setExpandedIds(prev => {
      const next = new Set(prev);
      if (isExpanded) {
        next.add(itemId);
      } else {
        next.delete(itemId);
      }
      return next;
    });
  }, []);

  return { expandedIds, handleExpandChange } as const;
}

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
  const debounceTimerRef = useRef<ReturnType<typeof setTimeout>>(null);

  useEffect(() => {
    return () => {
      if (debounceTimerRef.current) clearTimeout(debounceTimerRef.current);
    };
  }, []);

  // Compute visible entries and store in atom after commit
  const visibleEntries = useMemo(
    () => collectVisibleEntries([rootItem], expandedIds, selectedTypes, entities),
    [rootItem, expandedIds, selectedTypes, entities]
  );
  useEffect(() => {
    store.set(visibleEntriesAtom, visibleEntries);
  }, [visibleEntries, store]);

  // Read debounced zoom range reactively — drives the useQuery key.
  const debouncedZoomRange = useAtomValue(debouncedZoomRangeAtom);

  // Full fetch fires on mount and whenever the debounced zoomRange settles.
  const { data: bulkData } = useQuery({
    queryKey: ['bulkTimelines', engineId, queryId, debouncedZoomRange],
    queryFn: () => {
      const entries = store.get(visibleEntriesAtom);
      const windowSec = debouncedZoomRange.end - debouncedZoomRange.start;
      return fetchBulkTimelines(engineId, queryId, {
        num_bins: getAdaptiveNumBins(windowSec),
        start: debouncedZoomRange.start,
        end: debouncedZoomRange.end,
        entries,
      });
    },
    staleTime: DEFAULT_STALE_TIME,
  });

  // Distribute bulk results to per-item atoms.
  useEffect(() => {
    if (!bulkData) return;
    for (const [id, entry] of Object.entries(bulkData.resources)) {
      if (entry?.status === 'ok') {
        store.set(timelineDataAtom(id), bulkEntryToTimelineResponse(entry.data, bulkData.config));
      }
    }
    store.set(bulkInitializedAtom, true);
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

  // Expand handler — reads from store imperatively
  const handleExpand = useCallback(
    async (itemId: string, isExpanded: boolean) => {
      if (!isExpanded) return;

      const item = findItemById(rootItem, itemId);
      if (!item?.children) return;

      const newEntries: Record<string, BulkTimelineRequestParams> = {};
      for (const child of item.children) {
        if (!store.get(timelineDataAtom(child.id))) {
          newEntries[child.id] = buildBulkParamsForItem(child, selectedTypes, entities);
        }
      }

      if (Object.keys(newEntries).length === 0) return;

      const zoom = store.get(debouncedZoomRangeAtom);
      const windowSec = zoom.end - zoom.start;
      try {
        const response = await fetchBulkTimelines(engineId, queryId, {
          num_bins: getAdaptiveNumBins(windowSec),
          start: zoom.start,
          end: zoom.end,
          entries: newEntries,
        });
        for (const [id, entry] of Object.entries(response.resources)) {
          if (entry?.status === 'ok') {
            store.set(
              timelineDataAtom(id),
              bulkEntryToTimelineResponse(entry.data, response.config)
            );
          }
        }
      } catch {
        // Individual ResourceTimeline components will fall back to self-fetch
      }
    },
    [rootItem, selectedTypes, entities, engineId, queryId, store]
  );

  // Re-fetch a single item (e.g. after resource type change)
  const invalidateItem = useCallback(
    async (itemId: string) => {
      store.set(timelineDataAtom(itemId), undefined);
    },
    [store]
  );

  return { handleZoomChange, handleExpand, invalidateItem } as const;
}
