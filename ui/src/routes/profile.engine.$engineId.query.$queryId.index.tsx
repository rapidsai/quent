import { QueryResourceTree } from '@/components/QueryResourceTree';
import { queryBundleQueryOptions } from '@/hooks/useQueryBundle';
import { queryClient } from '@/lib/queryClient';
import { createFileRoute } from '@tanstack/react-router';
import { QueryBundle } from '~quent/types/QueryBundle';

export const Route = createFileRoute('/profile/engine/$engineId/query/$queryId/')({
  component: QueryIndex,
  loader: async ({ params }): Promise<QueryBundle> => {
    const { engineId, queryId } = params;
    // Use ensureQueryData to populate React Query cache (avoids duplicate fetches)
    return await queryClient.ensureQueryData(queryBundleQueryOptions({ engineId, queryId }));
  },
});

function QueryIndex() {
  const queryBundle = Route.useLoaderData();
  const { engineId } = Route.useParams();
  return (
    <div className="flex items-center justify-center w-full h-full min-h-[200px]">
      <QueryResourceTree engineId={engineId} queryBundle={queryBundle} />
    </div>
  );
}
