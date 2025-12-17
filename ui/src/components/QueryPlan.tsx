import { DAGChart } from '@/components/dag/DAGChart';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
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
      <Card className="transition-all hover:shadow-lg w-full">
        <CardHeader>
          <CardTitle className="text-lg">Query Execution Plan</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="w-full h-[600px]">
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
        </CardContent>
      </Card>

      {/* Query Plan DAG */}
      <div className="col-span-full"></div>
    </div>
  );
}
