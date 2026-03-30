// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { DEFAULT_STALE_TIME, fetchQueryBundle } from '@/services/api';
import { queryOptions, useQuery } from '@tanstack/react-query';
import type { QueryBundle } from '~quent/types/QueryBundle';
import type { EntityRef } from '~quent/types/EntityRef';

interface QueryBundleParams {
  engineId: string;
  queryId: string;
}

/**
 * Query options factory for queryBundle.
 * Use this in route loaders to pre-populate the cache,
 * and in useQueryBundle hook to read from the same cache.
 */
export const queryBundleQueryOptions = ({ engineId, queryId }: QueryBundleParams) =>
  queryOptions({
    queryKey: ['queryBundle', engineId, queryId],
    queryFn: async (): Promise<QueryBundle<EntityRef>> => {
      return fetchQueryBundle(engineId, queryId) as Promise<QueryBundle<EntityRef>>;
    },
    staleTime: DEFAULT_STALE_TIME,
    retry: 2,
  });

export const useQueryBundle = ({ engineId, queryId }: QueryBundleParams) => {
  return useQuery(queryBundleQueryOptions({ engineId, queryId }));
};
