// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { queryOptions, useQuery } from '@tanstack/react-query';
import { fetchListCoordinators } from './api';
import { DEFAULT_STALE_TIME } from './constants';

export const queryGroupsQueryOptions = (engineId: string, options?: { staleTime?: number }) =>
  queryOptions({
    queryKey: ['list_coordinators', engineId],
    queryFn: () => fetchListCoordinators(engineId),
    staleTime: options?.staleTime ?? DEFAULT_STALE_TIME,
    enabled: !!engineId,
  });

export const useQueryGroups = (engineId: string, options?: { staleTime?: number }) =>
  useQuery(queryGroupsQueryOptions(engineId, options));
