import { QueryResourceTree } from '@/components/QueryResourceTree';
import { fetchQueryBundle } from '@/services/api';
import { createFileRoute } from '@tanstack/react-router';
import { QueryBundle } from '~quent/types/QueryBundle';

export const Route = createFileRoute('/profile/engine/$engineId/query/$queryId/')({
  component: QueryIndex,
  loader: async ({ params }): Promise<QueryBundle> => {
    const { engineId, queryId } = params;
    return await fetchQueryBundle(engineId, queryId);
  },
});

function QueryIndex() {
  const { resource_tree, entities } = Route.useLoaderData();
  const { engineId } = Route.useParams();
  return (
    <div className="flex items-center justify-center w-full h-full min-h-[200px]">
      <QueryResourceTree engineId={engineId} resourceTree={resource_tree} entities={entities} />
    </div>
  );
}
