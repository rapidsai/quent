import { DAGChart } from '@/components/dag/DAGChart';
import { useQueryPlan } from '@/hooks/useQueryPlan';

export function QueryPlan({ queryId, engineId }: { queryId: string; engineId: string }) {
  // Load query plan (auto-detect format)
  const {
    data: queryPlanData,
    isLoading: queryPlanLoading,
    error: queryPlanError,
  } = useQueryPlan({
    source: {
      type: 'api',
      engineId,
      queryId,
    },
  });

  return (
    <div className="w-full space-y-8">
      <div className="w-full h-[calc(100vh-4rem)]">
        {queryPlanLoading ? (
          <div className="flex justify-center items-center h-full text-muted-foreground">
            Loading query plan...
          </div>
        ) : queryPlanError ? (
          <div className="flex justify-center items-center h-full text-destructive">
            Failed to load query plan:{' '}
            {queryPlanError instanceof Error ? queryPlanError.message : 'Unknown error'}
          </div>
        ) : queryPlanData ? (
          <DAGChart data={queryPlanData} queryId={queryId} engineId={engineId} height="100%" />
        ) : null}
      </div>

      {/* Query Plan DAG */}
      <div className="col-span-full"></div>
    </div>
  );
}
