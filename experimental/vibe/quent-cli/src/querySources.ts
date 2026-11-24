// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createHash } from 'node:crypto';
import {
  createHttpApiClient,
  fetchEntityList,
  fetchListEngines,
  fetchListQueries,
  fetchQueryBundle,
  setApiBaseUrl,
  type ApiClient,
} from '@quent/client';
import {
  parseJsonWithBigInt,
  type Engine,
  type EngineListResponse,
  type Query,
  type QueryListResponse,
} from '@quent/utils';
import type { DiscoveryApi, SelectFromList } from './askSelection';
import { inputWithInk } from './inkSelectorRunner';
import { launchQuentOpenDbRuns, stopQuentOpenDbRuns, type QuentOpenDbHandle } from './quentOpenDb';

export type QuerySourceApi = DiscoveryApi & Pick<ApiClient, 'fetchEntityList'>;

export interface QuerySource {
  id: string;
  label: string;
  api: QuerySourceApi;
}

export type QuerySourceCliValue = string | string[] | boolean | undefined;
export type QuerySourceCliValues = Record<string, QuerySourceCliValue>;
export type InputText = (
  prompt: string,
  placeholder?: string,
  signal?: AbortSignal
) => Promise<string>;

export const querySourceOptions = {
  'api-base': { type: 'string' as const },
  db: { type: 'boolean' as const },
  'db-api-base-url': { type: 'string' as const },
  'db-run': { type: 'string' as const, multiple: true },
  'db-token': { type: 'string' as const },
  'db-trust': { type: 'string' as const, multiple: true },
  'db-trust-all': { type: 'boolean' as const },
};

export const querySourceUsage = `  --api-base URL          Local Quent REST API; with --db-run, adds the local source
                          (default outside DB mode: QUENT_API_BASE_URL or http://localhost:8080/api)
  --db                    Prompt for comma-separated database run IDs
  --db-run RUN            Database run to add; repeat for multiple runs
  --db-api-base-url URL   Benchmarking API used to resolve --db-run
                          (default: QUENT_OPEN_API_BASE_URL)
  --db-token TOKEN        Benchmarking API bearer token (default: QUENT_OPEN_TOKEN)
  --db-trust REMOTE       Trust a git remote for quent-open; repeatable
  --db-trust-all          Trust every source opened from the database`;

export function optionalId(value: QuerySourceCliValue): string | undefined {
  return typeof value === 'string' && value.trim() !== '' ? value.trim() : undefined;
}

export function ids(value: QuerySourceCliValue): string[] {
  if (Array.isArray(value)) {
    return value.map(item => item.trim()).filter(Boolean);
  }
  const id = optionalId(value);
  return id ? [id] : [];
}

export async function inputDatabaseRunIds(
  prompt: string,
  placeholder?: string,
  signal?: AbortSignal
): Promise<string> {
  if (!process.stdin.isTTY || !process.stderr.isTTY) {
    throw new Error('Database run selection requires an interactive terminal; pass --db-run.');
  }
  return inputWithInk(prompt, placeholder, signal);
}

export async function resolveDatabaseRunIds(
  values: QuerySourceCliValues,
  inputText: InputText = inputDatabaseRunIds,
  signal?: AbortSignal
): Promise<string[]> {
  const explicit = ids(values['db-run']);
  if (explicit.length > 0) {
    return [...new Set(explicit)];
  }
  if (!values.db) {
    return [];
  }
  const entered = await inputText(
    'Enter database run IDs',
    'comma-separated, e.g. 6647, 6650',
    signal
  );
  const runs = entered
    .split(',')
    .map(run => run.trim())
    .filter(Boolean);
  if (runs.length === 0) {
    throw new Error('Enter at least one database run ID.');
  }
  return [...new Set(runs)];
}

type CatalogResponse<T> = {
  items: T[];
  initial_group_by_attribute: string | null;
};

function normalizeCatalogResponse<T>(value: unknown, name: string): CatalogResponse<T> {
  if (Array.isArray(value)) {
    return { items: value as T[], initial_group_by_attribute: null };
  }
  if (value && typeof value === 'object' && Array.isArray(Reflect.get(value, 'items'))) {
    return value as CatalogResponse<T>;
  }
  throw new Error(`The Quent API returned an invalid ${name} catalog.`);
}

function parseApiText<T>(text: string, response: Response): T {
  try {
    return parseJsonWithBigInt<T>(text);
  } catch {
    throw new Error(
      `Quent API endpoint ${response.url || '(unknown)'} returned a non-JSON response.`
    );
  }
}

async function parseApiResponse<T>(response: Response): Promise<T> {
  if (!response.ok) {
    throw new Error(`API Error: ${response.status} ${response.statusText}`);
  }
  return parseApiText<T>(await response.text(), response);
}

export function createHttpQuerySourceApi(baseUrl: string, signal?: AbortSignal): QuerySourceApi {
  const normalizedBaseUrl = baseUrl.replace(/\/+$/u, '');
  const client = createHttpApiClient(normalizedBaseUrl);
  const get = (path: string) => fetch(`${normalizedBaseUrl}${path}`, { signal });
  return {
    fetchListEngines: async (): Promise<EngineListResponse> =>
      normalizeCatalogResponse<Engine>(
        await parseApiResponse<unknown>(await get('/engines?with_metadata=true')),
        'engine'
      ),
    fetchListQueries: async (engineId: string): Promise<QueryListResponse> => {
      const encodedEngineId = encodeURIComponent(engineId);
      const response = await get(`/engines/${encodedEngineId}/queries`);
      if (response.ok) {
        const text = await response.text();
        if (!text.trimStart().startsWith('<')) {
          return normalizeCatalogResponse<Query>(parseApiText<unknown>(text, response), 'query');
        }
      } else if (response.status !== 404) {
        throw new Error(`API Error: ${response.status} ${response.statusText}`);
      }
      const groups = normalizeCatalogResponse<{ id: string }>(
        await parseApiResponse<unknown>(await get(`/engines/${encodedEngineId}/query-groups`)),
        'query group'
      );
      const groupedQueries = await Promise.all(
        groups.items.map(async group =>
          normalizeCatalogResponse<Query>(
            await parseApiResponse<unknown>(
              await get(
                `/engines/${encodedEngineId}/query_group/${encodeURIComponent(group.id)}/queries`
              )
            ),
            'query'
          )
        )
      );
      return {
        items: groupedQueries.flatMap(group => group.items),
        initial_group_by_attribute: null,
      };
    },
    fetchQueryBundle: async (engineId: string, queryId: string) =>
      parseApiResponse(
        await get(`/engines/${encodeURIComponent(engineId)}/query/${encodeURIComponent(queryId)}`)
      ),
    fetchEntityList: client.fetchEntityList,
  };
}

function globalQuerySourceApi(): QuerySourceApi {
  return {
    fetchListEngines,
    fetchListQueries,
    fetchQueryBundle,
    fetchEntityList,
  };
}

export function databaseQuerySources(
  handles: readonly QuentOpenDbHandle[],
  createApi: typeof createHttpQuerySourceApi = createHttpQuerySourceApi,
  signal?: AbortSignal
): QuerySource[] {
  const sources = handles.flatMap(handle =>
    handle.viewers.map(viewer => ({
      id:
        handle.viewers.length === 1
          ? handle.run
          : `${handle.run}#${createHash('sha256').update(viewer.label).digest('hex').slice(0, 16)}`,
      label:
        handle.viewers.length === 1 ? `db ${handle.run}` : `db ${handle.run} · ${viewer.label}`,
      api: createApi(viewer.apiBaseUrl, signal),
    }))
  );
  const sourceIds = sources.map(source => source.id);
  if (new Set(sourceIds).size !== sourceIds.length) {
    throw new Error('quent-open returned database viewers without unique labels.');
  }
  return sources;
}

export function findQuerySource(sources: readonly QuerySource[], id: string): QuerySource {
  const source = sources.find(candidate => candidate.id === id);
  if (!source) {
    throw new Error(
      `Unknown query source "${id}". Available sources: ${sources.map(candidate => candidate.id).join(', ')}.`
    );
  }
  return source;
}

export async function selectQuerySource(
  sources: readonly QuerySource[],
  requestedId: string | undefined,
  select: SelectFromList,
  prompt = 'Select a query source'
): Promise<QuerySource> {
  if (requestedId) {
    return findQuerySource(sources, requestedId);
  }
  if (sources.length === 1) {
    return sources[0]!;
  }
  if (sources.length === 0) {
    throw new Error('No query sources are available.');
  }
  const sourceId = await select(
    prompt,
    sources.map(source => ({ value: source.id, label: source.label }))
  );
  return findQuerySource(sources, sourceId);
}

export interface QuerySourceSession {
  sources: QuerySource[];
  databaseMode: boolean;
  signal: AbortSignal;
}

export interface QuerySourceRuntime {
  createApi?: typeof createHttpQuerySourceApi;
  launch: typeof launchQuentOpenDbRuns;
  stop: typeof stopQuentOpenDbRuns;
}

const defaultRuntime: QuerySourceRuntime = {
  launch: launchQuentOpenDbRuns,
  stop: stopQuentOpenDbRuns,
};

interface ReusableQuerySourceState {
  abortController: AbortController;
  handles: QuentOpenDbHandle[];
  interrupted: boolean;
  runtime?: QuerySourceRuntime;
  session?: QuerySourceSession;
  stopPromise?: Promise<void>;
}

let reusableState: ReusableQuerySourceState | undefined;

function stopReusableState(state: ReusableQuerySourceState): Promise<void> {
  if (!state.runtime) {
    return Promise.resolve();
  }
  state.stopPromise ??= state.runtime.stop(state.handles);
  return state.stopPromise;
}

export async function withReusableQuerySources<T>(run: () => Promise<T>): Promise<T> {
  if (reusableState) {
    throw new Error('A reusable query source session is already active.');
  }
  const state: ReusableQuerySourceState = {
    abortController: new AbortController(),
    handles: [],
    interrupted: false,
  };
  const stopOnSignal = (signal: NodeJS.Signals, exitCode: number) => {
    state.interrupted = true;
    process.exitCode = exitCode;
    state.abortController.abort();
    if (state.session || state.handles.length > 0) {
      void stopReusableState(state).finally(() => process.kill(process.pid, signal));
    }
  };
  const onInterrupt = () => stopOnSignal('SIGINT', 130);
  const onTerminate = () => stopOnSignal('SIGTERM', 143);
  reusableState = state;
  process.once('SIGINT', onInterrupt);
  process.once('SIGTERM', onTerminate);
  try {
    return await run();
  } finally {
    process.removeListener('SIGINT', onInterrupt);
    process.removeListener('SIGTERM', onTerminate);
    reusableState = undefined;
    await stopReusableState(state);
  }
}

export async function withQuerySources<T>(
  values: QuerySourceCliValues,
  inputText: InputText,
  run: (session: QuerySourceSession) => Promise<T>,
  runtime: QuerySourceRuntime = defaultRuntime
): Promise<T | undefined> {
  if (reusableState?.interrupted) {
    return undefined;
  }
  if (reusableState?.session) {
    return run(reusableState.session);
  }
  const configuredLocalApiBase = optionalId(values['api-base']) ?? process.env.QUENT_API_BASE_URL;
  const apiBase = configuredLocalApiBase ?? 'http://localhost:8080/api';
  setApiBaseUrl(apiBase.replace(/\/+$/u, ''));
  const abortController = reusableState?.abortController ?? new AbortController();
  const databaseRuns = await resolveDatabaseRunIds(values, inputText, abortController.signal);
  const databaseMode = Boolean(values.db) || databaseRuns.length > 0;
  const includeLocal = !databaseMode || configuredLocalApiBase !== undefined;

  let handles: QuentOpenDbHandle[] = [];
  let interrupted = false;
  let stopPromise: Promise<void> | undefined;
  const stopHandles = () => (stopPromise ??= runtime.stop(handles));
  const stopOnSignal = (signal: NodeJS.Signals, exitCode: number) => {
    interrupted = true;
    process.exitCode = exitCode;
    abortController.abort(new DOMException(`Interrupted by ${signal}.`, 'AbortError'));
    if (handles.length > 0) {
      void stopHandles();
    }
  };
  const onInterrupt = () => stopOnSignal('SIGINT', 130);
  const onTerminate = () => stopOnSignal('SIGTERM', 143);
  if (reusableState) {
    reusableState.runtime = runtime;
  } else if (databaseMode) {
    process.once('SIGINT', onInterrupt);
    process.once('SIGTERM', onTerminate);
  }
  try {
    if (databaseMode) {
      handles = await runtime.launch(databaseRuns, {
        apiBaseUrl: optionalId(values['db-api-base-url']),
        interactive: Boolean(!values.json && process.stdin.isTTY && process.stderr.isTTY),
        token: optionalId(values['db-token']),
        trust: ids(values['db-trust']),
        trustAll: Boolean(values['db-trust-all']),
        signal: abortController.signal,
      });
    }
    if (reusableState) {
      reusableState.handles = handles;
    }
    if (interrupted || reusableState?.interrupted) {
      return undefined;
    }
    abortController.signal.throwIfAborted();
    const createApi = runtime.createApi ?? createHttpQuerySourceApi;
    const sources: QuerySource[] = [
      ...(includeLocal
        ? [
            {
              id: 'local',
              label: 'local',
              api:
                databaseMode || runtime.createApi
                  ? createApi(apiBase.replace(/\/+$/u, ''), abortController.signal)
                  : globalQuerySourceApi(),
            },
          ]
        : []),
      ...databaseQuerySources(handles, createApi, abortController.signal),
    ];
    const session = { sources, databaseMode, signal: abortController.signal };
    if (reusableState) {
      reusableState.session = session;
    }
    return await run(session);
  } catch (error) {
    if (!abortController.signal.aborted) {
      throw error;
    }
    return undefined;
  } finally {
    if (databaseMode && !reusableState) {
      process.removeListener('SIGINT', onInterrupt);
      process.removeListener('SIGTERM', onTerminate);
      await stopHandles();
    }
  }
}
