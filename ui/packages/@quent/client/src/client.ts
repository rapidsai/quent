// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type {
  BulkTimelineRequest,
  BulkTimelinesResponse,
  DataFlowTimelineBinned,
  Engine,
  EngineContexts,
  EntityListRequest,
  EntityListResponse,
  EntityRef,
  NvtxCatalog,
  NvtxViewportRequest,
  NvtxViewportResponse,
  OperatorFilter,
  Query,
  QueryBundle,
  QueryFilter,
  QueryGroup,
  SingleTimelineRequest,
  SingleTimelineResponse,
  TimelineConfig,
} from '@quent/utils';

/** Typed operations consumed by the UI, independent of their transport. */
export interface ApiClient {
  fetchQueryBundle(engineId: string, queryId: string): Promise<QueryBundle<EntityRef>>;
  fetchListEngines(): Promise<Engine[]>;
  fetchEngineContexts(engineId: string): Promise<EngineContexts>;
  fetchNvtxCatalog(contextId: string, queryStartUnixNs: bigint): Promise<NvtxCatalog | null>;
  fetchNvtxViewport(
    contextId: string,
    queryStartUnixNs: bigint,
    request: NvtxViewportRequest
  ): Promise<NvtxViewportResponse | null>;
  fetchListCoordinators(engineId: string): Promise<QueryGroup[]>;
  fetchListQueries(engineId: string, coordinatorId: string): Promise<Query[]>;
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
