// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import init, { DemoServer } from '../../generated/simulator-wasm/quent_simulator_wasm.js';
import type { SimulatorWorkerRequest } from './simulatorProtocol';

let server: DemoServer | undefined;

self.onmessage = async ({ data }: MessageEvent<SimulatorWorkerRequest>) => {
  try {
    if (data.type === 'init') {
      await init();
      const response = await fetch(data.dataUrl);
      if (!response.ok) {
        throw new Error(`demo data request failed: ${response.status}`);
      }
      server = new DemoServer(new Uint8Array(await response.arrayBuffer()));
      self.postMessage({ id: data.id, response: '{}' });
      return;
    }
    if (!server) {
      throw new Error('simulator analyzer is not initialized');
    }
    let response: string;
    switch (data.operation) {
      case 'listEngines':
        response = await server.listEngines();
        break;
      case 'engineContexts':
        response = await server.engineContexts(data.engineId);
        break;
      case 'listCoordinators':
        response = await server.queryGroups(data.engineId);
        break;
      case 'listQueries':
        response = await server.queries(data.engineId, data.queryGroupId);
        break;
      case 'queryBundle':
        response = await server.query(data.engineId, data.queryId);
        break;
      case 'singleTimeline':
        response = await server.singleTimeline(data.engineId, JSON.stringify(data.request));
        break;
      case 'bulkTimelines':
        response = await server.bulkTimelines(data.engineId, JSON.stringify(data.request));
        break;
      case 'dataFlow':
        response = await server.dataFlowTimeline(data.engineId, JSON.stringify(data.request));
        break;
      case 'entityList':
        response = await server.entities(data.engineId, JSON.stringify(data.request));
        break;
    }
    self.postMessage({ id: data.id, response });
  } catch (error) {
    self.postMessage({ id: data.id, error: String(error) });
  }
};
