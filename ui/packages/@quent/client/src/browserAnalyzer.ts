// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { parseJsonWithBigInt } from '@quent/utils';
import type {
  BulkTimelineRequest,
  BulkTimelinesResponse,
  CategoricalTimelineRequest,
  DataFlowTimelineBinned,
  EngineContexts,
  EngineListResponse,
  EntityListRequest,
  EntityListResponse,
  EntityRef,
  OperatorFilter,
  QueryBundle,
  QueryFilter,
  QueryListResponse,
  SingleTimelineRequest,
  SingleTimelineResponse,
} from '@quent/utils';
import type { ApiClient } from './client';

export type BrowserAnalyzerOperation =
  | { operation: 'listEngines' }
  | { operation: 'engineContexts'; engineId: string }
  | { operation: 'listQueries'; engineId: string }
  | { operation: 'queryBundle'; engineId: string; queryId: string }
  | {
      operation: 'singleTimeline';
      engineId: string;
      request: SingleTimelineRequest<QueryFilter, OperatorFilter>;
    }
  | {
      operation: 'bulkTimelines';
      engineId: string;
      request: BulkTimelineRequest<QueryFilter, OperatorFilter>;
    }
  | {
      operation: 'dataFlow';
      engineId: string;
      request: CategoricalTimelineRequest<QueryFilter>;
    }
  | {
      operation: 'entityList';
      engineId: string;
      request: EntityListRequest<QueryFilter, OperatorFilter>;
    };

export type BrowserAnalyzerWorkerMessage =
  | { type: 'init'; moduleUrl: string; dataUrl: string }
  | ({ type: 'operation' } & BrowserAnalyzerOperation);

export type BrowserAnalyzerWorkerRequest = BrowserAnalyzerWorkerMessage & { id: number };

export type BrowserAnalyzerErrorCode =
  'invalid-request' | 'not-found' | 'unsupported' | 'analyzer-failure';

export interface BrowserAnalyzerErrorReply {
  code: BrowserAnalyzerErrorCode;
  message: string;
}

export type BrowserAnalyzerWorkerReply =
  { id: number; response: string } | { id: number; error: BrowserAnalyzerErrorReply };

export class BrowserAnalyzerError extends Error {
  constructor(
    readonly code: BrowserAnalyzerErrorCode,
    message: string
  ) {
    super(message);
    this.name = 'BrowserAnalyzerError';
  }
}

/** Convert a model-specific WASM exception into the shared worker error contract. */
export function browserAnalyzerErrorReply(error: unknown): BrowserAnalyzerErrorReply {
  const message = String(error);
  if (message.includes('QUENT_INVALID_REQUEST:')) {
    return {
      code: 'invalid-request',
      message: message.split('QUENT_INVALID_REQUEST:', 2)[1] ?? message,
    };
  }
  if (message.includes('QUENT_UNSUPPORTED:')) {
    return { code: 'unsupported', message: message.split('QUENT_UNSUPPORTED:', 2)[1] ?? message };
  }
  if (message.includes('QUENT_NOT_FOUND:')) {
    return { code: 'not-found', message: message.split('QUENT_NOT_FOUND:', 2)[1] ?? message };
  }
  return { code: 'analyzer-failure', message };
}

type WorkerPort = Pick<
  Worker,
  'postMessage' | 'onmessage' | 'onerror' | 'onmessageerror' | 'terminate'
>;

export interface BrowserAnalyzerClientOptions {
  moduleUrl: string;
  dataUrl: string;
}

/** Create the generic worker that hosts any standardized browser analyzer module. */
export function createBrowserAnalyzerWorker(): Worker {
  return new Worker(new URL('./browserAnalyzer.worker.mjs', import.meta.url), { type: 'module' });
}

/**
 * Construct an API client backed by a model-specific analyzer Web Worker.
 *
 * The worker owns WASM initialization and bundle decoding. The client keeps the
 * UI transport-neutral and preserves 64-bit values by parsing JSON with BigInt.
 */
export async function createBrowserAnalyzerClient(
  worker: WorkerPort,
  options: BrowserAnalyzerClientOptions
): Promise<ApiClient> {
  let nextId = 0;
  const pending = new Map<
    number,
    { resolve: (value: string) => void; reject: (reason: Error) => void }
  >();
  let workerError: Error | undefined;

  worker.onmessage = ({ data }: MessageEvent<BrowserAnalyzerWorkerReply>) => {
    const request = pending.get(data.id);
    if (!request) {
      return;
    }
    pending.delete(data.id);
    if ('error' in data) {
      request.reject(new BrowserAnalyzerError(data.error.code, data.error.message));
    } else {
      request.resolve(data.response);
    }
  };

  const rejectAll = (error: Error) => {
    workerError = error;
    for (const request of pending.values()) {
      request.reject(error);
    }
    pending.clear();
  };
  worker.onerror = event => rejectAll(new Error(event.message));
  worker.onmessageerror = () => rejectAll(new Error('unable to read analyzer worker response'));

  const send = (request: BrowserAnalyzerWorkerMessage) => {
    if (workerError) {
      return Promise.reject(workerError);
    }
    return new Promise<string>((resolve, reject) => {
      const id = nextId++;
      pending.set(id, { resolve, reject });
      worker.postMessage({ id, ...request });
    });
  };
  const call = async <T>(operation: BrowserAnalyzerOperation): Promise<T> =>
    parseJsonWithBigInt<T>(await send({ type: 'operation', ...operation }));

  await send({ type: 'init', ...options });

  return {
    fetchListEngines: () => call<EngineListResponse>({ operation: 'listEngines' }),
    fetchEngineContexts: engineId =>
      call<EngineContexts>({ operation: 'engineContexts', engineId }),
    fetchListQueries: engineId => call<QueryListResponse>({ operation: 'listQueries', engineId }),
    fetchQueryBundle: (engineId, queryId) =>
      call<QueryBundle<EntityRef>>({ operation: 'queryBundle', engineId, queryId }),
    fetchSingleTimeline: (engineId, request) =>
      call<SingleTimelineResponse>({ operation: 'singleTimeline', engineId, request }),
    fetchBulkTimelines: (engineId, request) =>
      call<BulkTimelinesResponse>({ operation: 'bulkTimelines', engineId, request }),
    fetchDataFlow: async (engineId, queryId, config, measures = []) => {
      try {
        return await call<DataFlowTimelineBinned>({
          operation: 'dataFlow',
          engineId,
          request: { measures, config, app_params: { query_id: queryId } },
        });
      } catch (error) {
        if (error instanceof BrowserAnalyzerError && error.code === 'unsupported') {
          return null;
        }
        throw error;
      }
    },
    fetchEntityList: (engineId, request) =>
      call<EntityListResponse>({ operation: 'entityList', engineId, request }),
    fetchNvtxCatalog: async () => null,
    fetchNvtxViewport: async () => null,
  };
}
