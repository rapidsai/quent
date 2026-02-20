import { useState, useEffect, lazy, Suspense } from 'react';
import { useQueryBundle } from '@/hooks/useQueryBundle';
import { useQueryPlanVisualization } from '@/hooks/useQueryPlanVisualization';
import { TreeView } from '@/components/ui/tree-view';
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from '@/components/ui/resizable';
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

  const singleQueryPlan = treeData.length === 1 && !treeData[0]?.children;

  const renderItem = ({ item, hasChildren }: { item: QueryPlanDataItem; hasChildren: boolean }) => {
    return (
      <div className="flex flex-col items-start py-0.5 pl-1">
        {singleQueryPlan ? (
          <span className="text-xs">Query: {item.queryId}</span>
        ) : (
          <span className="text-xs">
            <span className="capitalize">{item.planType}</span>
            {!hasChildren && <span>: {item.id}</span>}
          </span>
        )}
        {item.workerId && (
          <span className="text-xs text-muted-foreground">Worker: {item.workerId}</span>
        )}
        {hasChildren && (
          <span className="text-xs text-muted-foreground capitalize text-left">ID: {item.id}</span>
        )}
      </div>
    );
  };

  return (
    <div className="w-full flex flex-col h-[calc(100vh-4rem)]">
      <div className="flex items-center gap-2 px-4 py-1.5 border-b border-border bg-card shadow-sm flex-shrink-0">
        <Network className="h-4 w-4 text-primary" />
        <h3 className="text-xs font-semibold text-foreground">Query Plan Explorer</h3>
        <div className="text-xs text-muted-foreground">
          {queryBundle.entities.query_group.instance_name} -{' '}
          {queryBundle.entities.query.instance_name}
        </div>
      </div>

      <ResizablePanelGroup orientation="vertical" className="flex-1">
        <ResizablePanel
          defaultSize={20}
          minSize={5}
          className="overflow-y-auto [&::-webkit-scrollbar]:w-0 [scrollbar-width:none] [-ms-overflow-style:none]"
        >
          <TreeView<QueryPlanDataItem>
            data={treeData}
            initialSelectedItemId={planId}
            onSelectChange={handlePlanSelect}
            renderItem={renderItem}
          />
        </ResizablePanel>

        <ResizableHandle
          withHandle
          className="h-px w-full after:left-0 after:top-1/2 after:h-1 after:w-full after:-translate-y-1/2 after:translate-x-0 [&>div]:rotate-90 focus-visible:ring-0 focus-visible:ring-offset-0"
        />

        {/* DAG Chart - lazy loaded to split elkjs into separate chunk */}
        <ResizablePanel defaultSize={75} className="overflow-hidden">
          <Suspense
            fallback={
              <div className="flex items-center justify-center h-full text-muted-foreground">
                Loading visualization...
              </div>
            }
          >
            <DAGChart data={dagData} queryId={queryId} engineId={engineId} height="100%" />
          </Suspense>
        </ResizablePanel>
      </ResizablePanelGroup>
    </div>
  );
}
