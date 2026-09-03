// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { setApiClient, type ApiClient } from '@quent/client';
import { parseJsonWithBigInt } from '@quent/utils';
import type {
  DataFlowTimelineBinned,
  Engine,
  EngineContexts,
  EntityListResponse,
  Query,
  QueryBundle,
  QueryGroup,
  SingleTimelineResponse,
  BulkTimelinesResponse,
  EntityRef,
} from '@quent/utils';
import type {
  SimulatorOperation,
  SimulatorWorkerReply,
  SimulatorWorkerMessage,
} from './simulatorProtocol';

/** Start the simulator worker and install its typed operations as the UI API client. */
export async function installSimulatorClient(): Promise<void> {
  const worker = new Worker(new URL('./simulator.worker.ts', import.meta.url), { type: 'module' });
  let nextId = 0;
  const pending = new Map<
    number,
    { resolve: (value: string) => void; reject: (reason: Error) => void }
  >();
  let workerError: Error | undefined;

  worker.onmessage = ({ data }: MessageEvent<SimulatorWorkerReply>) => {
    const request = pending.get(data.id);
    if (!request) {
      return;
    }
    pending.delete(data.id);
    if ('error' in data) {
      request.reject(new Error(data.error));
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
  worker.onmessageerror = () => rejectAll(new Error('unable to read simulator worker response'));

  const send = (request: SimulatorWorkerMessage) => {
    if (workerError) {
      return Promise.reject(workerError);
    }
    return new Promise<string>((resolve, reject) => {
      const id = nextId++;
      pending.set(id, { resolve, reject });
      worker.postMessage({ id, ...request });
    });
  };
  const call = async <T>(operation: SimulatorOperation): Promise<T> =>
    parseJsonWithBigInt<T>(await send({ type: 'operation', ...operation }));

  const dataUrl = new URL('../../generated/simulator-demo.postcard', import.meta.url).href;
  await send({ type: 'init', dataUrl });

  const client: ApiClient = {
    fetchListEngines: () => call<Engine[]>({ operation: 'listEngines' }),
    fetchEngineContexts: engineId =>
      call<EngineContexts>({ operation: 'engineContexts', engineId }),
    fetchListCoordinators: engineId =>
      call<QueryGroup[]>({ operation: 'listCoordinators', engineId }),
    fetchListQueries: (engineId, queryGroupId) =>
      call<Query[]>({ operation: 'listQueries', engineId, queryGroupId }),
    fetchQueryBundle: (engineId, queryId) =>
      call<QueryBundle<EntityRef>>({ operation: 'queryBundle', engineId, queryId }),
    fetchSingleTimeline: (engineId, request) =>
      call<SingleTimelineResponse>({ operation: 'singleTimeline', engineId, request }),
    fetchBulkTimelines: (engineId, request) =>
      call<BulkTimelinesResponse>({ operation: 'bulkTimelines', engineId, request }),
    fetchDataFlow: (engineId, queryId, config, measures = []) =>
      call<DataFlowTimelineBinned>({
        operation: 'dataFlow',
        engineId,
        request: { measures, config, app_params: { query_id: queryId } },
      }),
    fetchEntityList: (engineId, request) =>
      call<EntityListResponse>({ operation: 'entityList', engineId, request }),
    fetchNvtxCatalog: async () => null,
    fetchNvtxViewport: async () => null,
  };
  setApiClient(client);
}
