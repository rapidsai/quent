// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { queryOptions, useQuery } from '@tanstack/react-query';
import type { SingleTimelineRequest, QueryFilter, TaskFilter } from '@quent/utils';
import { fetchSingleTimeline } from './api';
import { DEFAULT_STALE_TIME } from './constants';

interface SingleTimelineParams {
  engineId: string;
  request: SingleTimelineRequest<QueryFilter, TaskFilter>;
  durationSeconds: number;
}

export const singleTimelineQueryOptions = (
  { engineId, request, durationSeconds }: SingleTimelineParams,
  options?: { staleTime?: number }
) =>
  queryOptions({
    queryKey: ['singleTimeline', engineId, request, durationSeconds],
    queryFn: () => fetchSingleTimeline(engineId, request, durationSeconds),
    staleTime: options?.staleTime ?? DEFAULT_STALE_TIME,
  });

export const useTimeline = (
  params: SingleTimelineParams,
  options?: { staleTime?: number }
) => useQuery(singleTimelineQueryOptions(params, options));
