// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { parseJsonWithBigInt } from '@quent/utils';
import { getApiBaseUrl } from './config';
import type {
  QueryBundle,
  QueryGroup,
  Query,
  BulkTimelinesResponse,
  SingleTimelineRequest,
  SingleTimelineResponse,
  BulkTimelineRequest,
  DataFlowTimelineResponse,
  DistributionTimelineRequest,
  QueryFilter,
  OperatorFilter,
  EntityRef,
  Engine,
  TimelineConfig,
} from '@quent/utils';

interface ApiFetchOptions {
  params?: Record<string, string | number | boolean>;
  fetchOptions?: RequestInit;
}

/**
 * Generic API fetch helper — internal, not exported from package barrel
 * @param endpoint - API endpoint to call
 * @param options - Optional params and fetch options
 */
async function apiFetch<T>(endpoint: string, options?: ApiFetchOptions): Promise<T> {
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

  const response = await fetch(url, { ...defaultOptions, ...fetchOptions });

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
  return apiFetch<QueryBundle<EntityRef>>(`/engines/${engineId}/query/${queryId}`);
}

export async function fetchListEngines(): Promise<Engine[]> {
  return apiFetch<Engine[]>('/engines', { params: { with_metadata: true } });
}

export async function fetchListCoordinators(engineId: string): Promise<QueryGroup[]> {
  return apiFetch<QueryGroup[]>(`/engines/${engineId}/query-groups`);
}

export async function fetchListQueries(engineId: string, coordinatorId: string): Promise<Query[]> {
  return apiFetch<Query[]>(`/engines/${engineId}/query_group/${coordinatorId}/queries`);
}

export async function fetchSingleTimeline(
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

export async function fetchBulkTimelines(
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
 * Fetch the data-flow distribution timeline for a query (all operators in one
 * response). Returns `"Unsupported"` when the engine's analyzer does not
 * implement the data-flow protocol.
 * @param measures - Measure names to compute; empty means all declared measures.
 */
export async function fetchDataFlow(
  engineId: string,
  queryId: string,
  config: TimelineConfig,
  measures: string[] = []
): Promise<DataFlowTimelineResponse> {
  const request: DistributionTimelineRequest<QueryFilter> = {
    measures,
    config,
    app_params: { query_id: queryId },
  };
  return apiFetch<DataFlowTimelineResponse>(`/engines/${engineId}/timeline/data-flow`, {
    fetchOptions: {
      method: 'POST',
      body: JSON.stringify(request),
    },
  });
}
