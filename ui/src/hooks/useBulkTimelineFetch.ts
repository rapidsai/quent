import { useEffect, useMemo } from 'react';
import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { useStore } from 'jotai';
import { fetchBulkTimelines, DEFAULT_STALE_TIME } from '@/services/api';
import type { ZoomRange } from '@/components/timeline/TimelineController';
import type { TimelineRequest } from '~quent/types/TimelineRequest';
import type { TaskFilter } from '~quent/types/TaskFilter';
import { getResourceTypeName, setOperatorOnEntry } from '@/lib/timeline.utils';
import type { BulkTimelinesResponse } from '~quent/types/BulkTimelinesResponse';
import { timelineCacheKey, timelineDataAtom } from '@/atoms/timeline';

export interface BulkTimelineIdMeta {
  resourceId: string;
  resourceTypeName: string;
  operatorId: string | null;
}

export interface MergedBulkEntries {
  entries: Record<string, TimelineRequest<TaskFilter>>;
  idToMeta: Map<string, BulkTimelineIdMeta>;
  requestKey: string;
}

/**
 * Distributes a bulk timeline response into per-item Jotai atoms.
 * Skips entries whose status is not 'ok' or whose id has no meta mapping.
 */
export function applyBulkTimelineResponse(
  response: BulkTimelinesResponse,
  idToMeta: Map<string, BulkTimelineIdMeta>,
  store: ReturnType<typeof import('jotai').useStore>
): void {
  for (const [id, entry] of Object.entries(response.entries)) {
    if (entry?.status !== 'ok') continue;
    const meta = idToMeta.get(id);
    if (!meta) continue;
    const key = timelineCacheKey(meta.resourceId, meta.resourceTypeName, meta.operatorId);
    store.set(timelineDataAtom(key), {
      data: entry.data,
      config: entry.config,
    });
  }
}

/**
 * Builds a single bulk request: each resource appears once (base) and once with
 * operatorId when provided. Returns UUID-keyed entries, idToMeta map, and a stable requestKey.
 */
export function buildMergedBulkEntries(
  baseEntries: Record<string, TimelineRequest<TaskFilter>>,
  operatorId: string | null | undefined
): MergedBulkEntries {
  const entries: Record<string, TimelineRequest<TaskFilter>> = {};
  const idToMeta = new Map<string, BulkTimelineIdMeta>();

  for (const [resourceId, params] of Object.entries(baseEntries)) {
    const resourceTypeName = getResourceTypeName(params);
    const baseUuid = crypto.randomUUID();
    entries[baseUuid] = params;
    idToMeta.set(baseUuid, {
      resourceId,
      resourceTypeName,
      operatorId: null,
    });
    if (operatorId) {
      const opUuid = crypto.randomUUID();
      const withOperator = setOperatorOnEntry(params, operatorId);
      entries[opUuid] = withOperator;
      idToMeta.set(opUuid, {
        resourceId,
        resourceTypeName,
        operatorId,
      });
    }
  }

  const requestKey = JSON.stringify({
    keys: Object.keys(baseEntries).sort(),
    operatorId: operatorId ?? null,
  });

  return { entries, idToMeta, requestKey };
}

/**
 * Fetches bulk timeline data. Accepts base entries (keyed by resourceId) and optional
 * operatorId; builds merged entries (base + operator variants) internally and distributes
 * results to per-item Jotai atoms.
 */
export function useBulkTimelineFetch({
  engineId,
  queryId,
  debouncedZoomRange,
  entries,
  operatorId = null,
  enabled = true,
}: {
  engineId: string;
  queryId: string;
  debouncedZoomRange: ZoomRange;
  entries: Record<string, TimelineRequest<TaskFilter>>;
  operatorId?: string | null;
  enabled?: boolean;
}) {
  const store = useStore();

  const {
    entries: mergedEntries,
    idToMeta,
    requestKey,
  } = useMemo(() => buildMergedBulkEntries(entries, operatorId), [entries, operatorId]);

  const { data } = useQuery<BulkTimelinesResponse>({
    queryKey: ['bulkTimelines', engineId, queryId, debouncedZoomRange, requestKey],
    queryFn: () =>
      fetchBulkTimelines(engineId, {
        entries: mergedEntries,
        app_params: { query_id: queryId },
      }),
    staleTime: DEFAULT_STALE_TIME,
    enabled,
    placeholderData: keepPreviousData,
  });

  useEffect(() => {
    if (!data) return;
    applyBulkTimelineResponse(data, idToMeta, store);
  }, [data, store, idToMeta]);

  return data;
}
