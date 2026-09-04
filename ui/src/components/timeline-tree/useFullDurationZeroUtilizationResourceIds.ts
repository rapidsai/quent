// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { buildBulkParamsForItem, type TreeTableItem } from '@quent/components';
import { isTimelineUtilizationAllZero } from '@quent/hooks';
import { bulkTimelineQueryOptions } from '@quent/client';
import {
  EntityTypeKey,
  type OperatorFilter,
  type QueryEntities,
  type TimelineRequest,
} from '@quent/utils';

const EMPTY_SELECTED_TYPES = new Map<string, string>();

/**
 * Resource ids with zero utilization across the *entire* query duration —
 * fetched once, independent of zoom/scrub, so the caller can hide UI that
 * only makes sense when the resource has usages anywhere in the query
 * (e.g. the per-resource long-entities lane) without it jumping in and out
 * as the user scrubs the timeline.
 */
export function useFullDurationZeroUtilizationResourceIds(
  engineId: string,
  queryId: string,
  durationSeconds: number,
  entities: QueryEntities,
  enabled = true
): ReadonlySet<string> {
  const resourceIds = useMemo(() => Object.keys(entities.resources), [entities.resources]);

  const entries = useMemo(() => {
    const config = { num_bins: 1, start: 0, end: durationSeconds };
    const result: Record<string, TimelineRequest<OperatorFilter>> = {};
    for (const resourceId of resourceIds) {
      const item: TreeTableItem = {
        id: resourceId,
        type: EntityTypeKey.Resource,
        entity: entities.resources[resourceId]!,
      };
      result[resourceId] = buildBulkParamsForItem(item, EMPTY_SELECTED_TYPES, entities, config);
    }
    return result;
  }, [resourceIds, entities, durationSeconds]);

  const { data } = useQuery({
    ...bulkTimelineQueryOptions(
      { engineId, request: { entries, app_params: { query_id: queryId } } },
      { staleTime: Infinity }
    ),
    enabled: enabled && resourceIds.length > 0,
  });

  return useMemo(() => {
    const zeroResourceIds = new Set<string>();
    if (!data) {
      return zeroResourceIds;
    }
    for (const [resourceId, entry] of Object.entries(data.entries)) {
      if (entry.status === 'ok' && isTimelineUtilizationAllZero(entry.data)) {
        zeroResourceIds.add(resourceId);
      }
    }
    return zeroResourceIds;
  }, [data]);
}
