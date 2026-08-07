// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  infiniteQueryOptions,
  keepPreviousData,
  queryOptions,
  useInfiniteQuery,
  useQuery,
} from '@tanstack/react-query';
import type {
  EntityListRequest,
  EntityScope,
  EntitySortKey,
  OperatorFilter,
  QueryFilter,
  SortDir,
} from '@quent/utils';
import { fetchEntityList } from './api';
import { DEFAULT_STALE_TIME } from './constants';

interface EntityListParams {
  engineId: string;
  queryId: string;
  /** Window bounds in seconds relative to the query epoch. */
  window: { start: number; end: number };
  /** Restrict entities to the selected operators; empty returns entities across all. */
  operatorIds?: string[];
  /** Restrict entities to a resource / resource-group scope; `null` for all. */
  filter?: { scope?: EntityScope | null; entityTypeName?: string | null };
  /** Keep only entities whose longest usage span exceeds this (seconds). */
  minUsageSeconds?: number | null;
  sortKey?: EntitySortKey;
  sortDir?: SortDir;
  /** Max entities to return; omit for the full (unpaged) list. */
  maxItems?: number | null;
  /** Zero-based page index; only used when `maxItems` is set. */
  page?: number;
}

function buildRequest({
  queryId,
  window,
  operatorIds = [],
  filter,
  minUsageSeconds = null,
  sortKey = 'UsageDuration',
  sortDir = 'Desc',
  maxItems = null,
  page = 0,
}: EntityListParams): EntityListRequest<QueryFilter, OperatorFilter> {
  return {
    entry: {
      window,
      filter: {
        scope: filter?.scope ?? null,
        entity_type_name: filter?.entityTypeName ?? null,
        min_usage_s: minUsageSeconds,
      },
      sort: { key: sortKey, dir: sortDir },
      page: maxItems != null ? { page, max: maxItems } : null,
      application: { operator_ids: operatorIds },
    },
    app_params: { query_id: queryId },
  };
}

export const entityListQueryOptions = (
  params: EntityListParams,
  options?: { staleTime?: number; enabled?: boolean }
) => {
  const request = buildRequest(params);
  return queryOptions({
    queryKey: ['entityList', params.engineId, request],
    queryFn: () => fetchEntityList(params.engineId, request),
    staleTime: options?.staleTime ?? DEFAULT_STALE_TIME,
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
};
export const useEntityList = (
  params: EntityListParams,
  options?: { staleTime?: number; enabled?: boolean }
) => useQuery(entityListQueryOptions(params, options));

type PaginatedEntityListParams = EntityListParams & { maxItems: number };

export const entityListInfiniteQueryOptions = (
  params: PaginatedEntityListParams,
  options?: { staleTime?: number; enabled?: boolean }
) => {
  const initialRequest = buildRequest({ ...params, page: 0 });
  return infiniteQueryOptions({
    queryKey: ['entityList', 'infinite', params.engineId, initialRequest],
    queryFn: ({ pageParam }) =>
      fetchEntityList(params.engineId, buildRequest({ ...params, page: pageParam })),
    initialPageParam: 0,
    getNextPageParam: (lastPage, pages) => {
      const loadedCount = pages.reduce((count, page) => count + page.items.length, 0);
      return lastPage.items.length > 0 && loadedCount < lastPage.total ? pages.length : undefined;
    },
    staleTime: options?.staleTime ?? DEFAULT_STALE_TIME,
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
};

export const useInfiniteEntityList = (
  params: PaginatedEntityListParams,
  options?: { staleTime?: number; enabled?: boolean }
) => useInfiniteQuery(entityListInfiniteQueryOptions(params, options));
