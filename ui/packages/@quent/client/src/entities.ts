// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { keepPreviousData, queryOptions, useQuery } from '@tanstack/react-query';
import type { EntityListRequest, QueryFilter, OperatorFilter } from '@quent/utils';
import { fetchEntityList } from './api';
import { DEFAULT_STALE_TIME } from './constants';

interface EntitiesParams {
  engineId: string;
  request: EntityListRequest<QueryFilter, OperatorFilter>;
}

interface EntitiesOptions {
  staleTime?: number;
  enabled?: boolean;
}

export const entitiesQueryOptions = (
  { engineId, request }: EntitiesParams,
  options?: EntitiesOptions
) =>
  queryOptions({
    queryKey: ['entities', engineId, request],
    queryFn: () => fetchEntityList(engineId, request),
    staleTime: options?.staleTime ?? DEFAULT_STALE_TIME,
    enabled: options?.enabled,
    // Keep the current page visible while the next page/filter result loads.
    placeholderData: keepPreviousData,
  });

export const useEntities = (params: EntitiesParams, options?: EntitiesOptions) =>
  useQuery(entitiesQueryOptions(params, options));
