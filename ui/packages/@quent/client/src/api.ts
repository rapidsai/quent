// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { parseJsonWithBigInt } from '@quent/utils';
import { getApiClient } from './client';
import { getApiBaseUrl } from './config';
import { canonicalizeNvtxRequest } from './nvtxCanonical';
import type {
  QueryBundle,
  QueryGroup,
  Query,
  BulkTimelinesResponse,
  SingleTimelineRequest,
  SingleTimelineResponse,
  BulkTimelineRequest,
  CategoricalTimelineRequest,
  DataFlowTimelineBinned,
  QueryFilter,
  OperatorFilter,
  EntityRef,
  Engine,
  TimelineConfig,
  EntityListRequest,
  EntityListResponse,
  EngineContexts,
  NvtxCatalog,
  NvtxViewportRequest,
  NvtxViewportResponse,
} from '@quent/utils';

interface ApiFetchOptions {
  params?: Record<string, string | number | bigint | boolean>;
  fetchOptions?: RequestInit;
}

/**
 * Issues the request and returns the raw {@link Response} — internal helper
 * for fetchers that need to inspect the status code themselves.
 * @param endpoint - API endpoint to call
 * @param options - Optional params and fetch options
 */
async function apiFetchResponse(endpoint: string, options?: ApiFetchOptions): Promise<Response> {
  const { params, fetchOptions } = options ?? {};
  const searchParams = params
    ? `?${new URLSearchParams(Object.entries(params).map(([k, v]) => [k, String(v)]))}`
    : '';
  const url = `${getApiBaseUrl()}${endpoint}${searchParams}`;

  const defaultOptions: RequestInit = {
    headers: {},
  };

  // Only set Content-Type for requests with a body
  if (fetchOptions?.body) {
    defaultOptions.headers = {
      'Content-Type': 'application/json',
    };
  }

  return fetch(url, { ...defaultOptions, ...fetchOptions });
}

/**
 * Generic API fetch helper — internal, not exported from package barrel
 * @param endpoint - API endpoint to call
 * @param options - Optional params and fetch options
 */
async function apiFetch<T>(endpoint: string, options?: ApiFetchOptions): Promise<T> {
  const response = await apiFetchResponse(endpoint, options);

  if (!response.ok) {
    throw new Error(`API Error: ${response.status} ${response.statusText}`);
  }

  const text = await response.text();
  return parseJsonWithBigInt<T>(text);
}

/**
 * Fetch query bundle from API endpoint
 * @param engineId - The engine ID
 * @param queryId - The query ID to fetch the bundle for
 */
export async function fetchQueryBundle(
  engineId: string,
  queryId: string
): Promise<QueryBundle<EntityRef>> {
  const client = getApiClient();
  if (client) {
    return client.fetchQueryBundle(engineId, queryId);
  }
  return apiFetch<QueryBundle<EntityRef>>(`/engines/${engineId}/query/${queryId}`);
}

export async function fetchListEngines(): Promise<Engine[]> {
  const client = getApiClient();
  if (client) {
    return client.fetchListEngines();
  }
  return apiFetch<Engine[]>('/engines', { params: { with_metadata: true } });
}

export async function fetchEngineContexts(engineId: string): Promise<EngineContexts> {
  const client = getApiClient();
  if (client) {
    return client.fetchEngineContexts(engineId);
  }
  return apiFetch<EngineContexts>(`/engines/${engineId}/contexts`);
}

/** Fetch stable NVTX metadata, resolving a 404 to optional absence. */
export async function fetchNvtxCatalog(
  contextId: string,
  queryStartUnixNs: bigint
): Promise<NvtxCatalog | null> {
  const client = getApiClient();
  if (client) {
    return client.fetchNvtxCatalog(contextId, queryStartUnixNs);
  }
  const response = await apiFetchResponse(`/nvtx/contexts/${contextId}/catalog`, {
    params: { query_start: queryStartUnixNs },
  });
  if (response.status === 404) {
    return null;
  }
  if (!response.ok) {
    throw new Error(`API Error: ${response.status} ${response.statusText}`);
  }
  return parseJsonWithBigInt<NvtxCatalog>(await response.text());
}

export async function fetchNvtxViewport(
  contextId: string,
  queryStartUnixNs: bigint,
  request: NvtxViewportRequest
): Promise<NvtxViewportResponse | null> {
  const client = getApiClient();
  if (client) {
    return client.fetchNvtxViewport(contextId, queryStartUnixNs, request);
  }
  const canonical = canonicalizeNvtxRequest(request);
  const response = await apiFetchResponse(`/nvtx/contexts/${contextId}/viewport`, {
    params: { query_start: queryStartUnixNs },
    fetchOptions: {
      method: 'POST',
      body: JSON.stringify(canonical),
    },
  });
  if (response.status === 404) {
    return null;
  }
  if (!response.ok) {
    throw new Error(`API Error: ${response.status} ${response.statusText}`);
  }
  return normalizeNvtxViewport(parseJsonWithBigInt<NvtxViewportResponse>(await response.text()));
}

function asBigInt(value: bigint | number): bigint {
  return typeof value === 'bigint' ? value : BigInt(value);
}

function normalizeNvtxViewport(viewport: NvtxViewportResponse): NvtxViewportResponse {
  return {
    ...viewport,
    statistics: viewport.statistics.map(statistics => ({
      ...statistics,
      count: asBigInt(statistics.count),
      observed_count: asBigInt(statistics.observed_count),
    })),
  };
}

export async function fetchListCoordinators(engineId: string): Promise<QueryGroup[]> {
  const client = getApiClient();
  if (client) {
    return client.fetchListCoordinators(engineId);
  }
  return apiFetch<QueryGroup[]>(`/engines/${engineId}/query-groups`);
}

export async function fetchListQueries(engineId: string, coordinatorId: string): Promise<Query[]> {
  const client = getApiClient();
  if (client) {
    return client.fetchListQueries(engineId, coordinatorId);
  }
  return apiFetch<Query[]>(`/engines/${engineId}/query_group/${coordinatorId}/queries`);
}

export async function fetchSingleTimeline(
  engineId: string,
  request: SingleTimelineRequest<QueryFilter, OperatorFilter>,
  durationSeconds: number
): Promise<SingleTimelineResponse> {
  const client = getApiClient();
  if (client) {
    return client.fetchSingleTimeline(engineId, request, durationSeconds);
  }
  return apiFetch<SingleTimelineResponse>(`/engines/${engineId}/timeline/single`, {
    params: { duration: durationSeconds },
    fetchOptions: {
      method: 'POST',
      body: JSON.stringify(request),
    },
  });
}

export async function fetchBulkTimelines(
  engineId: string,
  request: BulkTimelineRequest<QueryFilter, OperatorFilter>
): Promise<BulkTimelinesResponse> {
  const client = getApiClient();
  if (client) {
    return client.fetchBulkTimelines(engineId, request);
  }
  return apiFetch<BulkTimelinesResponse>(`/engines/${engineId}/timeline/bulk`, {
    fetchOptions: {
      method: 'POST',
      body: JSON.stringify(request),
    },
  });
}

/**
 * Fetch a ranked, paged list of a query's entities (longest resource-usage
 * span first). Backs the long-entities Gantt view.
 */
export async function fetchEntityList(
  engineId: string,
  request: EntityListRequest<QueryFilter, OperatorFilter>
): Promise<EntityListResponse> {
  const client = getApiClient();
  if (client) {
    return client.fetchEntityList(engineId, request);
  }
  return apiFetch<EntityListResponse>(`/engines/${engineId}/entities`, {
    fetchOptions: {
      method: 'POST',
      body: JSON.stringify(request),
    },
  });
}

/**
 * Fetch the data-flow categorical timeline for a query (all operators in one
 * response). Resolves to `null` when the engine's analyzer does not implement
 * the data-flow protocol (HTTP 501) — an expected "feature unavailable"
 * outcome, not an error, so react-query settles instead of retrying.
 * @param measures - Measure names to compute; empty means all declared measures.
 */
export async function fetchDataFlow(
  engineId: string,
  queryId: string,
  config: TimelineConfig,
  measures: string[] = []
): Promise<DataFlowTimelineBinned | null> {
  const client = getApiClient();
  if (client) {
    return client.fetchDataFlow(engineId, queryId, config, measures);
  }
  const request: CategoricalTimelineRequest<QueryFilter> = {
    measures,
    config,
    app_params: { query_id: queryId },
  };
  const response = await apiFetchResponse(`/engines/${engineId}/timeline/data-flow`, {
    fetchOptions: {
      method: 'POST',
      body: JSON.stringify(request),
    },
  });
  if (response.status === 501) {
    return null;
  }
  if (!response.ok) {
    throw new Error(`API Error: ${response.status} ${response.statusText}`);
  }
  return parseJsonWithBigInt<DataFlowTimelineBinned>(await response.text());
}
