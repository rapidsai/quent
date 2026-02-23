import { HoverCard, HoverCardContent, HoverCardTrigger } from '@/components/ui/hover-card';
import { QueryPlanNodeData } from 'services/query-plan/types';

export interface OperatorStatisticsPopupProps {
  children: React.ReactNode;
  data: QueryPlanNodeData;
  nodeId: string;
  operatorLabel: string;
  operationType: string;
}

type StatValue = string | number | boolean | null;
type TaggedStatValue = Record<string, StatValue>;
type CustomStatistics = Record<string, TaggedStatValue>;

interface RawNodeStatistics {
  statistics?: {
    custom_statistics?: CustomStatistics;
  };
}

function parseCustomStatistics(rawNode: unknown): Array<{ key: string; value: StatValue }> {
  const statistics = (rawNode as RawNodeStatistics)?.statistics?.custom_statistics;
  if (!statistics) return [];

  return Object.entries(statistics).map(([key, tagged]) => ({
    key,
    value: Object.values(tagged)[0] ?? null,
  }));
}

export const OperatorStatisticsPopup = ({
  children,
  data,
  nodeId,
  operatorLabel,
  operationType,
}: OperatorStatisticsPopupProps) => {
  const stats = parseCustomStatistics(data.metadata?.rawNode);

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
        {stats.length > 0 && (
          <div className="mt-1 flex flex-col gap-1 border-t pt-1.5">
            {stats.map(({ key, value }) => (
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
