// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { keepPreviousData, queryOptions, useQuery } from '@tanstack/react-query';
import type { TimelineConfig } from '@quent/utils';
import { fetchDataFlow } from './api';
import { DEFAULT_STALE_TIME } from './constants';

interface DataFlowParams {
  engineId: string;
  queryId: string;
  /** Window (seconds relative to the query epoch) and bin count. */
  config: TimelineConfig;
  /** Measure names to compute; empty/omitted means all declared measures. */
  measures?: string[];
}

export const dataFlowQueryOptions = (
  { engineId, queryId, config, measures = [] }: DataFlowParams,
  options?: { staleTime?: number; enabled?: boolean }
) =>
  queryOptions({
    queryKey: ['dataFlow', engineId, queryId, config.start, config.end, config.num_bins, measures],
    // `fetchDataFlow` RESOLVES to `null` on HTTP 501 (analyzer without
    // data-flow support) rather than throwing, so react-query settles the
    // query as "unavailable" — no retries, no error noise.
    queryFn: () => fetchDataFlow(engineId, queryId, config, measures),
    staleTime: options?.staleTime ?? DEFAULT_STALE_TIME,
    enabled: options?.enabled ?? true,
    // Keep the previous window's data while a zoom-triggered refetch is in
    // flight so the DAG overlay doesn't flicker to empty.
    placeholderData: keepPreviousData,
  });

export const useDataFlow = (
  params: DataFlowParams,
  options?: { staleTime?: number; enabled?: boolean }
) => useQuery(dataFlowQueryOptions(params, options));
