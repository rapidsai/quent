// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type {
  BulkTimelineRequest,
  BulkTimelinesResponse,
  DataFlowTimelineBinned,
  EngineContexts,
  EngineListResponse,
  EntityListRequest,
  EntityListResponse,
  EntityRef,
  NvtxCatalog,
  NvtxViewportRequest,
  NvtxViewportResponse,
  OperatorFilter,
  QueryBundle,
  QueryFilter,
  QueryListResponse,
  SingleTimelineRequest,
  SingleTimelineResponse,
  TimelineConfig,
} from '@quent/utils';

/** Typed operations consumed by the UI, independent of their transport. */
export interface ApiClient {
  fetchQueryBundle(engineId: string, queryId: string): Promise<QueryBundle<EntityRef>>;
  fetchListEngines(): Promise<EngineListResponse>;
  fetchEngineContexts(engineId: string): Promise<EngineContexts>;
  fetchNvtxSinks(engineId: string): Promise<string[]>;
  fetchNvtxCatalog(
    engineId: string,
    sinkId: string,
    queryStartUnixNs: bigint
  ): Promise<NvtxCatalog | null>;
  fetchNvtxViewport(
    engineId: string,
    sinkId: string,
    queryStartUnixNs: bigint,
    request: NvtxViewportRequest
  ): Promise<NvtxViewportResponse | null>;
  fetchListQueries(engineId: string): Promise<QueryListResponse>;
  fetchSingleTimeline(
    engineId: string,
    request: SingleTimelineRequest<QueryFilter, OperatorFilter>,
    durationSeconds: number
  ): Promise<SingleTimelineResponse>;
  fetchBulkTimelines(
    engineId: string,
    request: BulkTimelineRequest<QueryFilter, OperatorFilter>
  ): Promise<BulkTimelinesResponse>;
  fetchEntityList(
    engineId: string,
    request: EntityListRequest<QueryFilter, OperatorFilter>
  ): Promise<EntityListResponse>;
  fetchDataFlow(
    engineId: string,
    queryId: string,
    config: TimelineConfig,
    measures?: string[]
  ): Promise<DataFlowTimelineBinned | null>;
}
