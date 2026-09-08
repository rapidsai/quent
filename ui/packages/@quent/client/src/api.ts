// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { parseJsonWithBigInt } from '@quent/utils';
import type { ApiClient } from './client';
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
async function httpFetchQueryBundle(
  engineId: string,
  queryId: string
): Promise<QueryBundle<EntityRef>> {
  return apiFetch<QueryBundle<EntityRef>>(`/engines/${engineId}/query/${queryId}`);
}

async function httpFetchListEngines(): Promise<Engine[]> {
  return apiFetch<Engine[]>('/engines', { params: { with_metadata: true } });
}

async function httpFetchEngineContexts(engineId: string): Promise<EngineContexts> {
  return apiFetch<EngineContexts>(`/engines/${engineId}/contexts`);
}

/** Fetch stable NVTX metadata, resolving a 404 to optional absence. */
async function httpFetchNvtxCatalog(
  contextId: string,
  queryStartUnixNs: bigint
): Promise<NvtxCatalog | null> {
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

async function httpFetchNvtxViewport(
  contextId: string,
  queryStartUnixNs: bigint,
  request: NvtxViewportRequest
): Promise<NvtxViewportResponse | null> {
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

async function httpFetchListCoordinators(engineId: string): Promise<QueryGroup[]> {
  return apiFetch<QueryGroup[]>(`/engines/${engineId}/query-groups`);
}

async function httpFetchListQueries(engineId: string, coordinatorId: string): Promise<Query[]> {
  return apiFetch<Query[]>(`/engines/${engineId}/query_group/${coordinatorId}/queries`);
}

async function httpFetchSingleTimeline(
  engineId: string,
  request: SingleTimelineRequest<QueryFilter, OperatorFilter>,
  durationSeconds: number
): Promise<SingleTimelineResponse> {
  return apiFetch<SingleTimelineResponse>(`/engines/${engineId}/timeline/single`, {
    params: { duration: durationSeconds },
    fetchOptions: {
      method: 'POST',
      body: JSON.stringify(request),
    },
  });
}

async function httpFetchBulkTimelines(
  engineId: string,
  request: BulkTimelineRequest<QueryFilter, OperatorFilter>
): Promise<BulkTimelinesResponse> {
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
async function httpFetchEntityList(
  engineId: string,
  request: EntityListRequest<QueryFilter, OperatorFilter>
): Promise<EntityListResponse> {
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
async function httpFetchDataFlow(
  engineId: string,
  queryId: string,
  config: TimelineConfig,
  measures: string[] = []
): Promise<DataFlowTimelineBinned | null> {
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

const httpClient: ApiClient = {
  fetchQueryBundle: httpFetchQueryBundle,
  fetchListEngines: httpFetchListEngines,
  fetchEngineContexts: httpFetchEngineContexts,
  fetchNvtxCatalog: httpFetchNvtxCatalog,
  fetchNvtxViewport: httpFetchNvtxViewport,
  fetchListCoordinators: httpFetchListCoordinators,
  fetchListQueries: httpFetchListQueries,
  fetchSingleTimeline: httpFetchSingleTimeline,
  fetchBulkTimelines: httpFetchBulkTimelines,
  fetchEntityList: httpFetchEntityList,
  fetchDataFlow: httpFetchDataFlow,
};

export const { getApiClient, setApiClient } = (() => {
  let activeClient: ApiClient = httpClient;

  return {
    getApiClient: (): ApiClient => activeClient,
    setApiClient: (client: ApiClient): void => {
      activeClient = client;
    },
  };
})();

export const fetchQueryBundle = (...args: Parameters<ApiClient['fetchQueryBundle']>) =>
  getApiClient().fetchQueryBundle(...args);
export const fetchListEngines = (...args: Parameters<ApiClient['fetchListEngines']>) =>
  getApiClient().fetchListEngines(...args);
export const fetchEngineContexts = (...args: Parameters<ApiClient['fetchEngineContexts']>) =>
  getApiClient().fetchEngineContexts(...args);
export const fetchNvtxCatalog = (...args: Parameters<ApiClient['fetchNvtxCatalog']>) =>
  getApiClient().fetchNvtxCatalog(...args);
export const fetchNvtxViewport = (...args: Parameters<ApiClient['fetchNvtxViewport']>) =>
  getApiClient().fetchNvtxViewport(...args);
export const fetchListCoordinators = (...args: Parameters<ApiClient['fetchListCoordinators']>) =>
  getApiClient().fetchListCoordinators(...args);
export const fetchListQueries = (...args: Parameters<ApiClient['fetchListQueries']>) =>
  getApiClient().fetchListQueries(...args);
export const fetchSingleTimeline = (...args: Parameters<ApiClient['fetchSingleTimeline']>) =>
  getApiClient().fetchSingleTimeline(...args);
export const fetchBulkTimelines = (...args: Parameters<ApiClient['fetchBulkTimelines']>) =>
  getApiClient().fetchBulkTimelines(...args);
export const fetchEntityList = (...args: Parameters<ApiClient['fetchEntityList']>) =>
  getApiClient().fetchEntityList(...args);
export const fetchDataFlow = (...args: Parameters<ApiClient['fetchDataFlow']>) =>
  getApiClient().fetchDataFlow(...args);
