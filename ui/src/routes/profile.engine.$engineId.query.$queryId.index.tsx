// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { QueryResourceTree } from '@/components/QueryResourceTree';
import { queryBundleQueryOptions } from '@/hooks/useQueryBundle';
import { queryClient } from '@/lib/queryClient';
import { useUrlStateSync } from '@/hooks/useUrlStateSync';
import { decodeTreeState } from '@/lib/treeStateParam';
import { createFileRoute } from '@tanstack/react-router';
import { QueryBundle } from '~quent/types/QueryBundle';
import type { EntityRef } from '~quent/types/EntityRef';

export const Route = createFileRoute('/profile/engine/$engineId/query/$queryId/')({
  validateSearch: (search: Record<string, unknown>) => ({
    planId: typeof search.planId === 'string' ? search.planId : undefined,
    operatorId: typeof search.operatorId === 'string' ? search.operatorId : undefined,
    operatorLabel: typeof search.operatorLabel === 'string' ? search.operatorLabel : undefined,
    zoomStart: Number.isFinite(Number(search.zoomStart)) ? Number(search.zoomStart) : undefined,
    zoomEnd: Number.isFinite(Number(search.zoomEnd)) ? Number(search.zoomEnd) : undefined,
    hideTasks:
      search.hideTasks === 'true' ? true : search.hideTasks === 'false' ? false : undefined,
    treeState: typeof search.treeState === 'string' ? search.treeState : undefined,
  }),
  component: QueryIndex,
  loader: async ({ params }): Promise<QueryBundle<EntityRef>> => {
    const { engineId, queryId } = params;
    // Use ensureQueryData to populate React Query cache (avoids duplicate fetches)
    return await queryClient.ensureQueryData(queryBundleQueryOptions({ engineId, queryId }));
  },
});

function QueryIndex() {
  const queryBundle = Route.useLoaderData();
  const { engineId } = Route.useParams();
  const search = Route.useSearch();

  useUrlStateSync(search);

  const initialZoom =
    search.zoomStart !== undefined && search.zoomEnd !== undefined
      ? { start: search.zoomStart, end: search.zoomEnd }
      : undefined;

  const initialTreeState = search.treeState ? decodeTreeState(search.treeState) : null;

  return (
    <div className="flex items-center justify-center w-full h-full min-h-[200px]">
      <QueryResourceTree
        engineId={engineId}
        queryBundle={queryBundle}
        initialZoom={initialZoom}
        initialTreeState={initialTreeState}
      />
    </div>
  );
}
