// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { browserAnalyzerErrorReply, type BrowserAnalyzerWorkerRequest } from './browserAnalyzer';

interface BrowserServer {
  listEngines(): Promise<string>;
  engineContexts(engineId: string): Promise<string>;
  queries(engineId: string): Promise<string>;
  query(engineId: string, queryId: string): Promise<string>;
  singleTimeline(engineId: string, request: string): Promise<string>;
  bulkTimelines(engineId: string, request: string): Promise<string>;
  dataFlowTimeline(engineId: string, request: string): Promise<string>;
  entities(engineId: string, request: string): Promise<string>;
}

interface BrowserAnalyzerModule {
  default(): Promise<unknown>;
  BrowserServer: new (bytes: Uint8Array) => BrowserServer;
}

let server: BrowserServer | undefined;

self.onmessage = async ({ data }: MessageEvent<BrowserAnalyzerWorkerRequest>) => {
  try {
    if (data.type === 'init') {
      // The module URL is selected by the hosting application. Vite must leave
      // it dynamic so separately built model analyzers can be deployed beside
      // the static UI without rebuilding Quent.
      const module = (await import(/* @vite-ignore */ data.moduleUrl)) as BrowserAnalyzerModule;
      await module.default();
      const response = await fetch(data.dataUrl);
      if (!response.ok) {
        throw new Error(`browser bundle request failed: ${response.status}`);
      }
      server = new module.BrowserServer(new Uint8Array(await response.arrayBuffer()));
      self.postMessage({ id: data.id, response: '{}' });
      return;
    }
    if (!server) {
      throw new Error('browser analyzer is not initialized');
    }

    let response: string;
    switch (data.operation) {
      case 'listEngines':
        response = await server.listEngines();
        break;
      case 'engineContexts':
        response = await server.engineContexts(data.engineId);
        break;
      case 'listQueries':
        response = await server.queries(data.engineId);
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
    self.postMessage({ id: data.id, error: browserAnalyzerErrorReply(error) });
  }
};
