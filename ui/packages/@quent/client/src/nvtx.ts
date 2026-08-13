// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { keepPreviousData, queryOptions, useQuery } from '@tanstack/react-query';
import type { NvtxCatalog, NvtxViewportRequest } from '@quent/utils';
import { fetchEngineContexts, fetchNvtxCatalog, fetchNvtxViewport } from './api';
import { DEFAULT_STALE_TIME } from './constants';
export { canonicalizeNvtxRequest, canonicalizeNvtxSelections } from './nvtxCanonical';

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
