import { createFileRoute, Outlet } from '@tanstack/react-router';
import { queryBundleQueryOptions } from '@/hooks/useQueryBundle';
import { queryClient } from '@/lib/queryClient';
import type { QueryBundle } from '~quent/types/QueryBundle';
import type { EntityRef } from '~quent/types/EntityRef';

export const Route = createFileRoute('/profile/engine/$engineId/query/$queryId')({
  component: QueryLayout,
  loader: async ({ params }): Promise<QueryBundle<EntityRef>> => {
    const { engineId, queryId } = params;
    return await queryClient.ensureQueryData(queryBundleQueryOptions({ engineId, queryId }));
  },
});

function QueryLayout() {
  return (
    <div className="flex flex-col h-full w-full">
      <div className="flex-1 min-h-0">
        <Outlet />
      </div>
    </div>
  );
}
