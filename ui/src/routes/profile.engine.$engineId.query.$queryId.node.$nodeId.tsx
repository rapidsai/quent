import { createFileRoute } from '@tanstack/react-router';
import { NodeProfile } from '@/components/NodeProfile';
import { fetchQueryBundle } from '@/services/api';
import { ResourceTree } from '~quent/types/ResourceTree';

// TODO: This does the same thing as the /query/$queryId route, figure out what happens when selecting nodes in the DAG
export const Route = createFileRoute('/profile/engine/$engineId/query/$queryId/node/$nodeId')({
  component: NodeRoute,
  loader: async ({ params }): Promise<ResourceTree> => {
    const { queryId, engineId } = params;
    const queryBundle = await fetchQueryBundle(engineId, queryId);
    return queryBundle.resource_tree;
  },
});

function NodeRoute() {
  const { engineId } = Route.useParams();
  const resourceTree = Route.useLoaderData();

  return <NodeProfile engineId={engineId} resourceTree={resourceTree} />;
}
