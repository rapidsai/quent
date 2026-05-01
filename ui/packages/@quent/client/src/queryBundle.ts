// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { queryOptions, useQuery } from '@tanstack/react-query';
import type { QueryBundle, EntityRef } from '@quent/utils';
import { fetchQueryBundle } from './api';
import { DEFAULT_STALE_TIME } from './constants';

interface QueryBundleParams {
  engineId: string;
  queryId: string;
}

export const queryBundleQueryOptions = ({ engineId, queryId }: QueryBundleParams) =>
  queryOptions({
    queryKey: ['queryBundle', engineId, queryId],
    queryFn: async (): Promise<QueryBundle<EntityRef>> =>
      fetchQueryBundle(engineId, queryId) as Promise<QueryBundle<EntityRef>>,
    staleTime: DEFAULT_STALE_TIME,
    retry: 2,
  });

export const useQueryBundle = (params: QueryBundleParams, options?: { staleTime?: number }) =>
  useQuery({
    ...queryBundleQueryOptions(params),
    staleTime: options?.staleTime ?? DEFAULT_STALE_TIME,
  });
