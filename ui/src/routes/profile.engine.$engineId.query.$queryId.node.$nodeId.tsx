// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createFileRoute } from '@tanstack/react-router';
import { QueryResourceTree } from '@/components/QueryResourceTree';
import { queryBundleQueryOptions } from '@/hooks/useQueryBundle';
import { queryClient } from '@/lib/queryClient';
import { QueryBundle } from '~quent/types/QueryBundle';
import type { EntityRef } from '~quent/types/EntityRef';

// TODO: This does the same thing as the /query/$queryId route, figure out what happens when selecting nodes in the DAG
export const Route = createFileRoute('/profile/engine/$engineId/query/$queryId/node/$nodeId')({
  component: NodeRoute,
  loader: async ({ params }): Promise<QueryBundle<EntityRef>> => {
    const { queryId, engineId } = params;
    return await queryClient.ensureQueryData(queryBundleQueryOptions({ engineId, queryId }));
  },
});

function NodeRoute() {
  const { engineId } = Route.useParams();
  const queryBundle = Route.useLoaderData();

  return <QueryResourceTree engineId={engineId} queryBundle={queryBundle} />;
}
