import { HoverCard, HoverCardContent, HoverCardTrigger } from '@/components/ui/hover-card';
import { StatValue } from '@/services/query-plan/types';

export interface OperatorStatisticsPopupProps {
  children: React.ReactNode;
  data: Array<{ key: string; value: StatValue }>;
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
        {data.length > 0 && (
          <div className="mt-1 flex flex-col gap-1 border-t pt-1.5">
            {data.map(({ key, value }) => (
              <div key={key} className="flex items-center justify-between text-xs">
                <span className="capitalize">{key.replace(/_/g, ' ')}:</span>
                <span className="text-muted-foreground ml-1 font-mono">{String(value)}</span>
              </div>
            ))}
          </div>
        )}
      </HoverCardContent>
    </HoverCard>
  );
};
