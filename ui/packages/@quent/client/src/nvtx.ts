// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { keepPreviousData, queryOptions, useQueries, useQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import type {
  NvtxCatalog,
  NvtxDomainSelection,
  NvtxViewportRequest,
  NvtxViewportWindow,
} from '@quent/utils';
import { fetchEngineContexts, fetchNvtxCatalog, fetchNvtxViewport } from './api';
import { DEFAULT_STALE_TIME } from './constants';
export { canonicalizeNvtxRequest, canonicalizeNvtxSelections } from './nvtxCanonical';

function compareDecimalIds(left: string, right: string): number {
  const a = BigInt(left);
  const b = BigInt(right);
  if (a < b) {
    return -1;
  }
  if (a > b) {
    return 1;
  }
  return 0;
}

/** Every catalog domain/category, matching the server's initial UI selection. */
export function selectAllNvtxDomains(catalog: Pick<NvtxCatalog, 'domains'>): NvtxDomainSelection[] {
  return catalog.domains
    .flatMap(domain => {
      const category_ids = domain.categories.map(category => category.category_id);
      if (category_ids.length === 0 && !domain.has_uncategorized) {
        return [];
      }
      return [
        {
          domain_id: domain.domain_id,
          category_ids,
          include_uncategorized: domain.has_uncategorized,
        },
      ];
    })
    .sort((left, right) => compareDecimalIds(left.domain_id, right.domain_id));
}

export interface NvtxCategoryFilter {
  categoryId: number | null;
  includeUncategorized: boolean;
}

/** Visible domains with each domain's optional category filter applied. */
export function selectNvtxDomains(
  catalog: Pick<NvtxCatalog, 'domains'>,
  domainId: string | null,
  categoryFilters: ReadonlyMap<string, NvtxCategoryFilter> = new Map()
): NvtxDomainSelection[] {
  const selections = selectAllNvtxDomains(catalog);
  return selections
    .filter(selection => domainId == null || selection.domain_id === domainId)
    .flatMap(selection => {
      const filter = categoryFilters.get(selection.domain_id);
      if (!filter) {
        return [selection];
      }
      if (filter.categoryId != null) {
        return selection.category_ids.includes(filter.categoryId)
          ? [{ ...selection, category_ids: [filter.categoryId], include_uncategorized: false }]
          : [];
      }
      return filter.includeUncategorized && selection.include_uncategorized
        ? [{ ...selection, category_ids: [], include_uncategorized: true }]
        : [];
    });
}

export const engineContextsQueryOptions = (engineId: string) =>
  queryOptions({
    queryKey: ['engineContexts', engineId],
    queryFn: () => fetchEngineContexts(engineId),
    staleTime: DEFAULT_STALE_TIME,
  });

export const nvtxCatalogStaleTime = (catalog: NvtxCatalog | null | undefined) =>
  catalog === null ? 0 : Infinity;

export const nvtxCatalogQueryOptions = (contextId: string, queryStartUnixNs: bigint) => {
  const queryStartKey = queryStartUnixNs.toString(10);
  return queryOptions({
    queryKey: ['nvtxCatalog', contextId, queryStartKey],
    queryFn: () => fetchNvtxCatalog(contextId, queryStartUnixNs),
    // A present catalog is immutable, but the server deliberately leaves an absent
    // stream retryable so telemetry that appears later can be discovered on remount.
    staleTime: query => nvtxCatalogStaleTime(query.state.data),
  });
};

export const nvtxViewportQueryOptions = (
  contextId: string,
  queryStartUnixNs: bigint,
  request: NvtxViewportRequest,
  options?: { enabled?: boolean; staleTime?: number }
) => {
  const queryStartKey = queryStartUnixNs.toString(10);
  const selectionKey = request.selections.map(selection => [
    selection.domain_id,
    [...selection.category_ids],
    selection.include_uncategorized,
  ]);
  return queryOptions({
    queryKey: [
      'nvtxViewport',
      contextId,
      queryStartKey,
      request.viewport.start,
      request.viewport.end,
      selectionKey,
    ],
    // Validation and canonicalization belong to the fetch boundary, where a bad
    // request rejects the query rather than throwing during React render.
    queryFn: () => fetchNvtxViewport(contextId, queryStartUnixNs, request),
    enabled: options?.enabled ?? true,
    staleTime: options?.staleTime ?? DEFAULT_STALE_TIME,
    placeholderData: keepPreviousData,
  });
};

export const useEngineContexts = (engineId: string) =>
  useQuery(engineContextsQueryOptions(engineId));

export const useNvtxCatalog = (contextId: string, queryStartUnixNs: bigint) =>
  useQuery(nvtxCatalogQueryOptions(contextId, queryStartUnixNs));

export const useNvtxViewport = (
  contextId: string,
  queryStartUnixNs: bigint,
  request: NvtxViewportRequest,
  options?: { enabled?: boolean; staleTime?: number }
) => useQuery(nvtxViewportQueryOptions(contextId, queryStartUnixNs, request, options));

/** First context whose catalog request returned a stream. */
export function firstNvtxCatalog(
  contextIds: string[],
  catalogs: Array<NvtxCatalog | null | undefined>
): { contextId: string; catalog: NvtxCatalog } | null {
  for (let index = 0; index < contextIds.length; index++) {
    const catalog = catalogs[index];
    if (catalog != null) {
      return { contextId: contextIds[index]!, catalog };
    }
  }
  return null;
}

/** Resolve the NVTX context for an engine and fetch catalog + viewport. */
export function useNvtxStream(
  engineId: string,
  queryStartUnixNs: bigint,
  viewport: NvtxViewportWindow,
  options?: {
    staleTime?: number;
    enabled?: boolean;
    domainId?: string | null;
    categoryFilters?: ReadonlyMap<string, NvtxCategoryFilter>;
  }
) {
  const contextsQuery = useEngineContexts(engineId);
  const contextIds = Object.keys(contextsQuery.data?.context_resources ?? {});
  const catalogQueries = useQueries({
    queries: contextIds.map(contextId => nvtxCatalogQueryOptions(contextId, queryStartUnixNs)),
  });
  const matched = firstNvtxCatalog(
    contextIds,
    catalogQueries.map(query => query.data)
  );
  const catalog = matched?.catalog ?? null;
  const contextId = matched?.contextId;
  const selections = useMemo(
    () =>
      catalog
        ? selectNvtxDomains(catalog, options?.domainId ?? null, options?.categoryFilters)
        : [],
    [catalog, options?.categoryFilters, options?.domainId]
  );
  const request = useMemo(
    (): NvtxViewportRequest => ({ viewport, selections }),
    [viewport, selections]
  );
  const viewportQuery = useNvtxViewport(contextId ?? '', queryStartUnixNs, request, {
    enabled: !!contextId && selections.length > 0 && (options?.enabled ?? true),
    staleTime: options?.staleTime,
  });
  const catalogsPending = contextIds.length > 0 && catalogQueries.some(query => query.isPending);
  return {
    contextId,
    catalog,
    viewport: viewportQuery.data ?? null,
    isLoading: contextsQuery.isLoading || catalogsPending,
  };
}
