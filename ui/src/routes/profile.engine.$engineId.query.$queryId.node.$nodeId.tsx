import { createFileRoute } from '@tanstack/react-router';
import { NodeProfile } from '@/components/NodeProfile';
import { fetchNodeProfile } from '@/services/api';

export interface NodeRouteData {
  nodeId: string;
  timestamps: number[];
  series: Record<string, number[]>;
}

export const Route = createFileRoute('/profile/engine/$engineId/query/$queryId/node/$nodeId')({
  component: NodeRoute,
  loader: async ({ params }): Promise<NodeRouteData> => {
    // Will be used to retrieve time series data for node
    const { nodeId, queryId } = params;
    const data = await fetchNodeProfile(queryId, nodeId);
    return data;
  },
});

function NodeRoute() {
  const { queryId, nodeId } = Route.useParams();
  const loaderData = Route.useLoaderData();

  return <NodeProfile nodeId={nodeId} queryId={queryId} data={loaderData} />;
}
