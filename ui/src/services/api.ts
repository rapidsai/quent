/**
 * API Service - Stubs for calling backend webservices
 * Replace these with actual API endpoints
 */

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || 'http://localhost:8000/api';

// Simulated delay for mock API calls
const delay = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

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

export async function fetchQuery(queryId: string): Promise<QueryResponse> {
  // return apiFetch<QueryResponse>(`/queries/${queryId}`);
  return { id: queryId };
}

export async function fetchNodeProfile(
  _queryId: string,
  nodeId: string
): Promise<NodeProfileResponse> {
  // return apiFetch<NodeProfileResponse>(`/nodes/${nodeId}/profile`);

  const timestamps: number[] = [];
  let base = +new Date(2001, 9, 3);
  const oneDay = 1 * 600 * 1000;
  const tempData = [Math.random() * 300];
  for (let i = 1; i < 500; i++) {
    const now = new Date((base += oneDay));
    timestamps.push(+now);
    tempData.push(Math.round((Math.random() - 0.5) * 20 + tempData[i - 1]));
  }
  const tempData2 = [Math.random() * 300];
  for (let i = 1; i < 500; i++) {
    tempData2.push(Math.round((Math.random() - 0.5) * 20 + tempData2[i - 1]));
  }
  const tempData3 = [Math.random() * 300];
  for (let i = 1; i < 500; i++) {
    tempData3.push(Math.round((Math.random() - 0.5) * 20 + tempData3[i - 1]));
  }

  await delay(200);
  return {
    nodeId,
    timestamps,
    series: { Mem: tempData, IO: tempData2, CPU: tempData3 },
  };
}

/**
 * Fetch line chart data
 * @returns Promise with array of data points
 */
export async function fetchLineChartData(): Promise<ChartDataPoint[]> {
  // TODO: Replace with actual API call
  // const response = await fetch(`${API_BASE_URL}/line-chart-data`);
  // return response.json();

  await delay(500);

  // Mock data
  const mockData: ChartDataPoint[] = [];
  const now = new Date();

  for (let i = 30; i >= 0; i--) {
    const date = new Date(now);
    date.setDate(date.getDate() - i);
    mockData.push({
      date: date.toISOString().split('T')[0],
      value: Math.floor(Math.random() * 100) + 50,
    });
  }

  return mockData;
}

/**
 * Fetch bar chart data
 * @returns Promise with array of bar chart data
 */
export async function fetchBarChartData(): Promise<BarChartData[]> {
  // TODO: Replace with actual API call
  // const response = await fetch(`${API_BASE_URL}/bar-chart-data`);
  // return response.json();

  await delay(500);

  // Mock data
  return [
    { category: 'Product A', value: 120 },
    { category: 'Product B', value: 200 },
    { category: 'Product C', value: 150 },
    { category: 'Product D', value: 80 },
    { category: 'Product E', value: 170 },
  ];
}

/**
 * Generic API fetch helper
 * @param endpoint - API endpoint to call
 * @param options - Fetch options
 */
export async function apiFetch<T>(endpoint: string, options?: RequestInit): Promise<T> {
  const url = `${API_BASE_URL}${endpoint}`;

  const defaultOptions: RequestInit = {
    headers: {},
  };

  // Only set Content-Type for requests with a body
  if (options?.body) {
    defaultOptions.headers = {
      'Content-Type': 'application/json',
    };
  }

  const response = await fetch(url, { ...defaultOptions, ...options });

  if (!response.ok) {
    throw new Error(`API Error: ${response.status} ${response.statusText}`);
  }

  return response.json();
}

/**
 * Fetch query plan from local JSON file
 * For development/testing purposes
 */
export async function fetchLocalQueryPlan(filename: string = 'plan.json'): Promise<unknown> {
  await delay(200);

  const response = await fetch(`/${filename}`);

  if (!response.ok) {
    throw new Error(`Failed to load query plan: ${response.statusText}`);
  }

  return response.json();
}

/**
 * Fetch query plan from API endpoint
 * @param queryId - The query ID to fetch the plan for
 */
export async function fetchQueryPlan(engineId: string, queryId: string): Promise<unknown> {
  return apiFetch<unknown>(`/engine/${engineId}/query/${queryId}`);
}

export async function fetchListEngines(): Promise<string[]> {
  return apiFetch<string[]>('/engine/list');
}

export async function fetchListCoordinators(engineId: string): Promise<string[]> {
  return apiFetch<string[]>(`/engine/${engineId}/query_groups`);
}

export async function fetchListQueries(engineId: string, coordinatorId: string): Promise<string[]> {
  return apiFetch<string[]>(`/engine/${engineId}/query_groups/${coordinatorId}/list_queries`);
}
