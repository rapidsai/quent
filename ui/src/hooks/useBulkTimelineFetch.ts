import { useEffect } from 'react';
import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { useStore } from 'jotai';
import { fetchBulkTimelines, DEFAULT_STALE_TIME } from '@/services/api';
import type { ZoomRange } from '@/components/timeline/TimelineController';
import type { TimelineRequest } from '~quent/types/TimelineRequest';
import type { TaskFilter } from '~quent/types/TaskFilter';
import {
  getAdaptiveNumBins,
  getResourceTypeName,
  setOperatorOnEntries,
} from '@/lib/timeline.utils';
import { timelineCacheKey, timelineDataAtom, visibleEntriesAtom } from '@/atoms/timeline';

/**
 * Fetches bulk timeline data and distributes results to per-item Jotai atoms.
 * When operatorId is provided, entries are transformed to include the operator filter.
 */
export function useBulkTimelineFetch({
  engineId,
  queryId,
  debouncedZoomRange,
  baseEntries,
  operatorId,
  enabled = true,
}: {
  engineId: string;
  queryId: string;
  debouncedZoomRange: ZoomRange;
  baseEntries: Record<string, TimelineRequest<TaskFilter>>;
  operatorId?: string | null;
  enabled?: boolean;
}) {
  const store = useStore();

  const { data } = useQuery({
    queryKey: ['bulkTimelines', engineId, queryId, debouncedZoomRange, operatorId, baseEntries],
    queryFn: () => {
      let entries = store.get(visibleEntriesAtom);
      if (operatorId) {
        entries = setOperatorOnEntries(entries, operatorId);
      }
      const windowSec = debouncedZoomRange.end - debouncedZoomRange.start;
      return fetchBulkTimelines(engineId, {
        config: {
          num_bins: getAdaptiveNumBins(windowSec),
          start: debouncedZoomRange.start,
          end: debouncedZoomRange.end,
        },
        entries,
        app_params: { query_id: queryId },
      });
    },
    staleTime: DEFAULT_STALE_TIME,
    enabled,
    placeholderData: keepPreviousData,
  });

  useEffect(() => {
    if (!data) return;
    const currentEntries = store.get(visibleEntriesAtom);
    for (const [id, entry] of Object.entries(data.entries)) {
      if (entry?.status === 'ok') {
        const resourceTypeName = getResourceTypeName(currentEntries[id]);
        store.set(timelineDataAtom(timelineCacheKey(id, resourceTypeName, operatorId)), {
          data: entry.data,
          config: data.config,
        });
      }
    }
  }, [data, store, operatorId]);

  return data;
}
