// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo, useState } from 'react';
import { createFileRoute, Link, Outlet } from '@tanstack/react-router';
import { queryBundleQueryOptions } from '@quent/client';
import { queryClient } from '@/lib/queryClient';
import type { QueryBundle, EntityRef } from '@quent/utils';
import { cn } from '@quent/utils';
import { QueryLoading } from '@/components/QueryLoading';
import { RouteError } from '@/components/RouteError';
import { TabActionSlotsContext, type TabActionSlots } from '@/components/TabActionSlots';
import { validateDeepLinkSearch } from '@/features/deep-link';

export const Route = createFileRoute('/profile/engine/$engineId/query/$queryId')({
  component: QueryLayout,
  errorComponent: RouteError,
  pendingComponent: QueryLoading,
  pendingMs: 200,
  pendingMinMs: 300,
  validateSearch: validateDeepLinkSearch,
  loader: async ({ params }): Promise<QueryBundle<EntityRef>> => {
    const { engineId, queryId } = params;
    return await queryClient.ensureQueryData(queryBundleQueryOptions({ engineId, queryId }));
  },
});

const tabClass = cn(
  'inline-flex items-center justify-center whitespace-nowrap rounded-md px-3 py-1',
  'text-sm font-normal text-muted-foreground transition-all',
  'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2'
);

const activeTabClass = cn(tabClass, 'text-foreground font-semibold bg-muted shadow');

function QueryLayout() {
  const { engineId, queryId } = Route.useParams();
  const [leftSlot, setLeftSlot] = useState<HTMLElement | null>(null);
  const [rightSlot, setRightSlot] = useState<HTMLElement | null>(null);
  const slots = useMemo<TabActionSlots>(() => ({ leftSlot, rightSlot }), [leftSlot, rightSlot]);
  return (
    <TabActionSlotsContext.Provider value={slots}>
      <div className="flex min-w-0 flex-col h-full w-full">
        <div className="shrink-0 border-b">
          <div className="grid h-9 w-full grid-cols-[1fr_auto_1fr] items-center gap-2 px-3 py-1 text-xs text-muted-foreground">
            <div ref={setLeftSlot} className="flex min-w-0 items-center gap-1.5" />
            <div className="flex items-center justify-center">
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
              <Link
                to="/profile/engine/$engineId/query/$queryId/entities"
                params={{ engineId, queryId }}
                className={tabClass}
                activeProps={{ className: activeTabClass }}
              >
                Entities
              </Link>
            </div>
            <div ref={setRightSlot} className="flex items-center justify-end gap-2" />
          </div>
        </div>
        <div className="min-w-0 flex-1 min-h-0">
          <Outlet />
        </div>
      </div>
    </TabActionSlotsContext.Provider>
  );
}
