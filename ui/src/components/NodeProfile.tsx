import type { NodeRouteData } from '@/routes/profile.engine.$engineId.query.$queryId.node.$nodeId';
import { Timeline } from './Timeline';

interface NodeProfileProps {
  nodeId: string;
  queryId: string;
  data: NodeRouteData;
}

export function NodeProfile({ queryId, data }: NodeProfileProps) {
  const { nodeId, timestamps, series } = data;

  return (
    <div className="space-y-4">
      <h2 className="text-lg font-semibold">{nodeId} Profile</h2>
      <Timeline timestamps={timestamps} series={series} />
      <div className="border rounded-lg p-4 bg-card">
        <dl className="space-y-2">
          <div>
            <dt className="text-sm font-medium text-muted-foreground">Node ID</dt>
            <dd className="text-sm">{nodeId}</dd>
          </div>
          <div>
            <dt className="text-sm font-medium text-muted-foreground">Query ID</dt>
            <dd className="text-sm">{queryId}</dd>
          </div>
        </dl>
      </div>
    </div>
  );
}
