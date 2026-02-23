import { HoverCard, HoverCardContent, HoverCardTrigger } from '@/components/ui/hover-card';
import { QueryPlanNodeData } from 'services/query-plan/types';

export interface OperatorStatisticsPopupProps {
  children: React.ReactNode;
  data: QueryPlanNodeData;
  nodeId: string;
  operatorLabel: string;
  operationType: string;
}

export const OperatorStatisticsPopup = ({
  children,
  data,
  nodeId,
  operatorLabel,
  operationType,
}: OperatorStatisticsPopupProps) => {
  const metadata = data.metadata.rawNode;
  console.log(metadata.statistics);
  return (
    <HoverCard openDelay={300} closeDelay={100}>
      {/* nodrag/nopan prevents ReactFlow from intercepting mouse events on the trigger */}
      <HoverCardTrigger asChild className="nodrag nopan">
        {children}
      </HoverCardTrigger>
      <HoverCardContent className="flex w-72 flex-col gap-1.5">
        <div className="flex items-center justify-between">
          <span className="font-semibold text-sm">{operatorLabel}</span>
          <span className="text-xs text-muted-foreground capitalize px-1.5 py-0.5 bg-muted rounded">
            {operationType}
          </span>
        </div>
        <div className="text-xs text-muted-foreground font-mono truncate">{nodeId}</div>
        {/* TODO: fetch and render operator statistics using nodeId, engineId, queryId */}
        <div className="text-xs text-muted-foreground mt-1">engine: … · query: …</div>
      </HoverCardContent>
    </HoverCard>
  );
};
