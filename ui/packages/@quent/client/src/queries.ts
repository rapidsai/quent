// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { queryOptions, useQuery } from '@tanstack/react-query';
import { fetchListQueries } from './api';
import { DEFAULT_STALE_TIME } from './constants';

export const queriesQueryOptions = (
  engineId: string,
  coordinatorId: string,
  options?: { staleTime?: number }
) =>
  queryOptions({
    queryKey: ['list_queries', engineId, coordinatorId],
    queryFn: () => fetchListQueries(engineId, coordinatorId),
    staleTime: options?.staleTime ?? DEFAULT_STALE_TIME,
    enabled: !!engineId && !!coordinatorId,
  });

export const useQueries = (
  engineId: string,
  coordinatorId: string,
  options?: { staleTime?: number }
) => useQuery(queriesQueryOptions(engineId, coordinatorId, options));
