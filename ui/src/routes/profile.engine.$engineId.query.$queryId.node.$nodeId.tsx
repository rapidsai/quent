import { createFileRoute } from '@tanstack/react-router';
import { QueryResourceTree } from '@/components/QueryResourceTree';
import { fetchQueryBundle } from '@/services/api';
import { QueryBundle } from '~quent/types/QueryBundle';

// TODO: This does the same thing as the /query/$queryId route, figure out what happens when selecting nodes in the DAG
export const Route = createFileRoute('/profile/engine/$engineId/query/$queryId/node/$nodeId')({
  component: NodeRoute,
  loader: async ({ params }): Promise<QueryBundle> => {
    const { queryId, engineId } = params;
    return await fetchQueryBundle(engineId, queryId);
  },
});

function NodeRoute() {
  const { engineId } = Route.useParams();
  const { resource_tree, entities } = Route.useLoaderData();

  return <QueryResourceTree engineId={engineId} resourceTree={resource_tree} entities={entities} />;
}
