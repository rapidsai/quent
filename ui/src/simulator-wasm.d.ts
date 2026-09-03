// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

declare module '*quent_simulator_wasm.js' {
  export default function init(): Promise<unknown>;
  export class DemoServer {
    constructor(bytes: Uint8Array);
    listEngines(): Promise<string>;
    engineContexts(engineId: string): Promise<string>;
    queryGroups(engineId: string): Promise<string>;
    queries(engineId: string, queryGroupId: string): Promise<string>;
    query(engineId: string, queryId: string): Promise<string>;
    singleTimeline(engineId: string, request: string): Promise<string>;
    bulkTimelines(engineId: string, request: string): Promise<string>;
    dataFlowTimeline(engineId: string, request: string): Promise<string>;
    entities(engineId: string, request: string): Promise<string>;
  }
}
