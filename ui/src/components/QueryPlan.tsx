import { useState, useEffect, lazy, Suspense } from 'react';
import { useQueryBundle } from '@/hooks/useQueryBundle';
import { useQueryPlanVisualization } from '@/hooks/useQueryPlanVisualization';
import { TreeView } from '@/components/ui/tree-view';
import { type QueryPlanDataItem } from '@/services/query-plan/types';
import { Network } from 'lucide-react';

// Lazy load DAGChart to split elkjs (~1.6MB) into a separate chunk
const DAGChart = lazy(() =>
  import('@/components/dag/DAGChart').then(mod => ({ default: mod.DAGChart }))
);

export function QueryPlan({ queryId, engineId }: { queryId: string; engineId: string }) {
  const [planId, setPlanId] = useState<string>('');

  const {
    data: queryBundle,
    isLoading: queryBundleLoading,
    error: queryBundleError,
  } = useQueryBundle({ engineId, queryId });

  const { dagData, treeData, error: dagError } = useQueryPlanVisualization(queryBundle, planId);

  const handlePlanSelect = (item: QueryPlanDataItem | undefined) => {
    if (item) {
      setPlanId(item.id);
    }
  };

  // TODO: Currently fetching root plan when bundle loads - is this correct?
  useEffect(() => {
    if (queryBundle && !planId) {
      setPlanId(queryBundle.plan_tree.id);
    }
  }, [queryBundle, planId]);

  // handle loading and error states
  if (queryBundleLoading) {
    return (
      <div className="w-full flex flex-col h-[calc(100vh-4rem)]">
        <div className="flex justify-center items-center h-full text-muted-foreground">
          Loading query plan...
        </div>
      </div>
    );
  }

  const errorMessage = queryBundleError
    ? `Failed to load query plan: ${queryBundleError instanceof Error ? queryBundleError.message : 'Unknown error'}`
    : dagError
      ? `Failed to generate query plan visualization: ${dagError.message}`
      : null;

  if (errorMessage) {
    return (
      <div className="w-full flex flex-col h-[calc(100vh-4rem)]">
        <div className="flex justify-center items-center h-full text-destructive">
          {errorMessage}
        </div>
      </div>
    );
  }

  if (!queryBundle || !planId) {
    return null;
  }

  const singleQuery = treeData.length === 1 && !treeData[0]?.children;

  return (
    <div className="w-full flex flex-col h-[calc(100vh-4rem)]">
      <div className="border-b border-border bg-card shadow-sm">
        <div className="flex items-center gap-2 px-4 py-2 border-b border-border">
          <Network className="h-4 w-4 text-primary" />
          <h3 className="text-sm font-semibold text-foreground">Query Plan Explorer</h3>
        </div>
        <div className="px-2 py-2 overflow-y-auto [&::-webkit-scrollbar]:w-0 [scrollbar-width:none] [-ms-overflow-style:none] max-h-40">
          <TreeView<QueryPlanDataItem>
            data={treeData}
            initialSelectedItemId={planId}
            onSelectChange={handlePlanSelect}
            renderItem={({ item, hasChildren }) => (
              <div className="flex flex-col py-0.5">
                {hasChildren || singleQuery ? (
                  <span className="text-sm">Query: {item.queryId}</span>
                ) : item.planType ? (
                  <span className="text-sm">
                    Query Plan ({item.planType}): {item.id}
                  </span>
                ) : null}
                {item.workerId && (
                  <span className="text-xs text-muted-foreground">Worker: {item.workerId}</span>
                )}
                {(singleQuery || hasChildren) && item.planType && (
                  <span className="text-xs text-muted-foreground capitalize text-left">
                    Plan Type: {item.planType}
                  </span>
                )}
              </div>
            )}
          />
        </div>
      </div>

      {/* DAG Chart - lazy loaded to split elkjs into separate chunk */}
      <div className="flex-1 overflow-hidden">
        <Suspense
          fallback={
            <div className="flex items-center justify-center h-full text-muted-foreground">
              Loading visualization...
            </div>
          }
        >
          <DAGChart data={dagData} queryId={queryId} engineId={engineId} height="100%" />
        </Suspense>
      </div>
    </div>
  );
}
