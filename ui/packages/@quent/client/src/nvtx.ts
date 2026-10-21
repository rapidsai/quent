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
import { fetchEngineContexts, fetchNvtxSinks, fetchNvtxCatalog, fetchNvtxViewport } from './api';
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

export const nvtxCatalogQueryOptions = (
  engineId: string,
  sinkId: string,
  queryStartUnixNs: bigint
) => {
  const queryStartKey = queryStartUnixNs.toString(10);
  return queryOptions({
    queryKey: ['nvtxCatalog', engineId, sinkId, queryStartKey],
    queryFn: () => fetchNvtxCatalog(engineId, sinkId, queryStartUnixNs),
    staleTime: Infinity,
  });
};

export const nvtxViewportQueryOptions = (
  engineId: string,
  sinkId: string,
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
      engineId,
      sinkId,
      queryStartKey,
      request.viewport.start,
      request.viewport.end,
      selectionKey,
    ],
    // Validation and canonicalization belong to the fetch boundary, where a bad
    // request rejects the query rather than throwing during React render.
    queryFn: () => fetchNvtxViewport(engineId, sinkId, queryStartUnixNs, request),
    enabled: options?.enabled ?? true,
    staleTime: options?.staleTime ?? DEFAULT_STALE_TIME,
    placeholderData: keepPreviousData,
  });
};

export const useEngineContexts = (engineId: string) =>
  useQuery(engineContextsQueryOptions(engineId));

export const useNvtxCatalog = (engineId: string, sinkId: string, queryStartUnixNs: bigint) =>
  useQuery(nvtxCatalogQueryOptions(engineId, sinkId, queryStartUnixNs));

export const useNvtxViewport = (
  engineId: string,
  sinkId: string,
  queryStartUnixNs: bigint,
  request: NvtxViewportRequest,
  options?: { enabled?: boolean; staleTime?: number }
) => useQuery(nvtxViewportQueryOptions(engineId, sinkId, queryStartUnixNs, request, options));

/** First sink whose catalog request returned a model. */
export function firstNvtxCatalog(
  sinkIds: string[],
  catalogs: Array<NvtxCatalog | null | undefined>
): { sinkId: string; catalog: NvtxCatalog } | null {
  for (let index = 0; index < sinkIds.length; index++) {
    const catalog = catalogs[index];
    if (catalog != null) {
      return { sinkId: sinkIds[index]!, catalog };
    }
  }
  return null;
}

/** Resolve an engine's NVTX sink and fetch its catalog and viewport. */
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
  const sinksQuery = useQuery({
    queryKey: ['nvtxSinks', engineId],
    queryFn: () => fetchNvtxSinks(engineId),
    staleTime: DEFAULT_STALE_TIME,
  });
  const sinkIds = sinksQuery.data ?? [];
  const catalogQueries = useQueries({
    queries: sinkIds.map(sinkId => nvtxCatalogQueryOptions(engineId, sinkId, queryStartUnixNs)),
  });
  const matched = firstNvtxCatalog(
    sinkIds,
    catalogQueries.map(query => query.data)
  );
  const catalog = matched?.catalog ?? null;
  const sinkId = matched?.sinkId;
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
  const viewportQuery = useNvtxViewport(engineId, sinkId ?? '', queryStartUnixNs, request, {
    enabled: !!sinkId && selections.length > 0 && (options?.enabled ?? true),
    staleTime: options?.staleTime,
  });
  const catalogsPending = sinkIds.length > 0 && catalogQueries.some(query => query.isPending);
  return {
    sinkId,
    catalog,
    viewport: viewportQuery.data ?? null,
    isLoading: sinksQuery.isLoading || catalogsPending,
  };
}
