// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createFileRoute, Link, Outlet } from '@tanstack/react-router';
import { queryBundleQueryOptions } from '@/hooks/useQueryBundle';
import { queryClient } from '@/lib/queryClient';
import type { QueryBundle } from '~quent/types/QueryBundle';
import type { EntityRef } from '~quent/types/EntityRef';
import { cn } from '@/lib/utils';

export const Route = createFileRoute('/profile/engine/$engineId/query/$queryId')({
  component: QueryLayout,
  loader: async ({ params }): Promise<QueryBundle<EntityRef>> => {
    const { engineId, queryId } = params;
    return await queryClient.ensureQueryData(queryBundleQueryOptions({ engineId, queryId }));
  },
});

const tabClass = cn(
  'inline-flex items-center justify-center whitespace-nowrap rounded-md px-3 py-1',
  'text-sm font-medium text-muted-foreground transition-all',
  'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2'
);

const activeTabClass = cn(tabClass, 'bg-background text-foreground shadow');

function QueryLayout() {
  const { engineId, queryId } = Route.useParams();
  return (
    <div className="flex flex-col h-full w-full">
      <div className="shrink-0 border-b">
        <div className="inline-flex h-9 w-full items-center justify-center bg-muted p-1 text-muted-foreground gap-0">
          <Link
            to="/profile/engine/$engineId/query/$queryId/timeline"
            params={{ engineId, queryId }}
            className={tabClass}
            activeProps={{ className: activeTabClass }}
          >
            Timeline
          </Link>
          <Link
            to="/profile/engine/$engineId/query/$queryId/operators"
            params={{ engineId, queryId }}
            className={tabClass}
            activeProps={{ className: activeTabClass }}
          >
            Operators
          </Link>
        </div>
      </div>
      <div className="flex-1 min-h-0">
        <Outlet />
      </div>
    </div>
  );
}
