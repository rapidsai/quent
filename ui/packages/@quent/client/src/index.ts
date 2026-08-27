// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// Constants
export { DEFAULT_STALE_TIME } from './constants';
export { getApiBaseUrl, setApiBaseUrl } from './config';

// Fetch functions
export {
  fetchQueryBundle,
  fetchListEngines,
  fetchListCoordinators,
  fetchListQueries,
  fetchSingleTimeline,
  fetchBulkTimelines,
  fetchDataFlow,
  fetchEntityList,
  fetchEngineContexts,
  fetchNvtxCatalog,
  fetchNvtxViewport,
} from './api';

// queryOptions factories
export { queryBundleQueryOptions } from './queryBundle';
export { enginesQueryOptions } from './engines';
export { queryGroupsQueryOptions } from './queryGroups';
export { queriesQueryOptions } from './queries';
export { singleTimelineQueryOptions } from './timeline';
export { bulkTimelineQueryOptions } from './bulkTimelines';
export { dataFlowQueryOptions } from './dataFlow';
export { entityListQueryOptions } from './entityList';
export {
  canonicalizeNvtxRequest,
  canonicalizeNvtxSelections,
  engineContextsQueryOptions,
  firstNvtxCatalog,
  nvtxCatalogQueryOptions,
  nvtxViewportQueryOptions,
  selectAllNvtxDomains,
} from './nvtx';
export type { NvtxCategoryFilter } from './nvtx';

// Hooks
export { useQueryBundle } from './queryBundle';
export { useEngines } from './engines';
export { useQueryGroups } from './queryGroups';
export { useQueries } from './queries';
export { useTimeline } from './timeline';
export { useDataFlow } from './dataFlow';
export { useEntityList } from './entityList';
export { useEngineContexts, useNvtxCatalog, useNvtxStream, useNvtxViewport } from './nvtx';
