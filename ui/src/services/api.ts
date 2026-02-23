/**
 * API Service - Stubs for calling backend webservices
 * Replace these with actual API endpoints
 */

import { QueryBundle } from '~quent/types/QueryBundle';
import { TimelineResponse } from '~quent/types/TimelineResponse';
import { QueryGroup } from '~quent/types/QueryGroup';
import { Query } from '~quent/types/Query';
import { BulkTimelinesRequest } from '~quent/types/BulkTimelinesRequest';
import { BulkTimelinesResponse } from '~quent/types/BulkTimelinesResponse';

// Use relative URL by default to leverage Vite's proxy (both dev and preview)
// Set VITE_API_BASE_URL to override (e.g., for direct API access without proxy)
const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || '/api';
export const DEFAULT_STALE_TIME = 5 * 60 * 1000;

/**
 * TODO: Figure out a more permanent solution for this
 * Parse JSON with BigInt support for large integers.
 * Integers larger than Number.MAX_SAFE_INTEGER are converted to BigInt.
 */
export function parseJsonWithBigInt<T>(text: string): T {
  // Match integers that are too large for Number (and not floats)
  // This regex finds: a number boundary, optional minus, digits only (no decimal/exponent)
  // We convert integers > MAX_SAFE_INTEGER to BigInt
  const processed = text.replace(
    /([:\s[,]|^)(-?\d{16,})(?=[,\s}\]]|$)/g,
    (match, prefix, numStr) => {
      const num = Number(numStr);
      // Only convert if it exceeds safe integer range
      if (!Number.isSafeInteger(num)) {
        return `${prefix}"__bigint__${numStr}"`;
      }
      return match;
    }
  );

  return JSON.parse(processed, (_key, value) => {
    if (typeof value === 'string' && value.startsWith('__bigint__')) {
      return BigInt(value.slice(10));
    }
    return value;
  });
}

export interface ChartDataPoint {
  date: string;
  value: number;
}

export interface BarChartData {
  category: string;
  value: number;
}

export interface DashboardMetrics {
  totalUsers: number;
  activeUsers: number;
  revenue: number;
  growth: number;
}

export interface QueryResponse {
  id: string;
}

export interface DAGResponse {
  queryId: string;
  nodes: DAGNode[];
  edges: DAGEdge[];
}

export interface DAGNode {
  id: string;
  name: string;
  type: string;
  label: string;
  details: DAGNodeDetails;
  lineage: DAGLineage[];
}

export interface DAGLineage {
  index: number;
  source: { database: string; table: string; column: string };
}

export interface DAGNodeDetails {
  table?: string[];
  columns?: { name: string; index: number }[];
  groupBy?: number[];
  aggregations?: { name: string; function: string; inputIndex: number }[];
}

export interface DAGEdge {
  id: string;
  source: string;
  target: string;
}

export type NodeProfileResponse = {
  nodeId: string;
  timestamps: number[];
  series: Record<string, number[]>;
};

interface ApiFetchOptions {
  params?: Record<string, string | number | boolean>;
  fetchOptions?: RequestInit;
}

/**
 * Generic API fetch helper
 * @param endpoint - API endpoint to call
 * @param options - Optional params and fetch options
 */
export async function apiFetch<T>(endpoint: string, options?: ApiFetchOptions): Promise<T> {
  const { params, fetchOptions } = options ?? {};
  const searchParams = params
    ? `?${new URLSearchParams(Object.entries(params).map(([k, v]) => [k, String(v)]))}`
    : '';
  const url = `${API_BASE_URL}${endpoint}${searchParams}`;

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
 * @param queryId - The query ID to fetch the bundle for
 */
export async function fetchQueryBundle(engineId: string, queryId: string): Promise<QueryBundle> {
  return apiFetch<QueryBundle>(`/engines/${engineId}/query/${queryId}`);
}

export async function fetchListEngines(): Promise<string[]> {
  return apiFetch<string[]>('/engines');
}

export async function fetchListCoordinators(engineId: string): Promise<QueryGroup[]> {
  return apiFetch<QueryGroup[]>(`/engines/${engineId}/query-groups`);
}

export async function fetchListQueries(engineId: string, coordinatorId: string): Promise<Query[]> {
  return apiFetch<Query[]>(`/engines/${engineId}/query_group/${coordinatorId}/queries`);
}

export async function fetchResourceTimeline(
  engineId: string,
  queryId: string,
  resourceId: string,
  params?: Record<string, string | number | boolean>
): Promise<TimelineResponse> {
  return apiFetch<TimelineResponse>(
    `/engines/${engineId}/query/${queryId}/resource/${resourceId}/timeline`,
    { params }
  );
}

export async function fetchResourceGroupTimeline(
  engineId: string,
  queryId: string,
  resourceGroupId: string,
  params?: Record<string, string | number | boolean>
): Promise<TimelineResponse> {
  return apiFetch<TimelineResponse>(
    `/engines/${engineId}/query/${queryId}/resource_group/${resourceGroupId}/timeline`,
    { params }
  );
}

export async function fetchBulkTimelines(
  engineId: string,
  queryId: string,
  request: BulkTimelinesRequest
): Promise<BulkTimelinesResponse> {
  return apiFetch<BulkTimelinesResponse>(`/engines/${engineId}/query/${queryId}/timelines`, {
    fetchOptions: {
      method: 'POST',
      body: JSON.stringify(request),
    },
  });
}
