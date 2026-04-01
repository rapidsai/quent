// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect, lazy, Suspense } from 'react';
import { useAtom, useSetAtom } from 'jotai';
import { useQueryBundle } from '@/hooks/useQueryBundle';
import { useQueryPlanVisualization } from '@/hooks/useQueryPlanVisualization';
import { TreeView } from '@/components/ui/tree-view';
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from '@/components/ui/resizable';
import { type QueryPlanDataItem } from '@/services/query-plan/types';
import { Network } from 'lucide-react';
import { selectedPlanIdAtom, hoveredWorkerIdAtom } from '@/atoms/dag';
import { DataText } from '@/components/ui/data-text';

// Lazy load DAGChart to split elkjs (~1.6MB) into a separate chunk
const DAGChart = lazy(() =>
  import('@/components/dag/DAGChart').then(mod => ({ default: mod.DAGChart }))
);

export function QueryPlan({ queryId, engineId }: { queryId: string; engineId: string }) {
  const [planId, setPlanId] = useAtom(selectedPlanIdAtom);
  const setHoveredWorkerId = useSetAtom(hoveredWorkerIdAtom);

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
  }, [queryBundle, planId, setPlanId]);

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
      <div
        className="flex flex-col items-start py-0.5 pl-1"
        onMouseEnter={() => item.workerId && setHoveredWorkerId(item.workerId)}
        onMouseLeave={() => setHoveredWorkerId(null)}
      >
        {singleQueryPlan ? (
          <span className="text-xs">
            Query: <DataText>{item.queryId}</DataText>
          </span>
        ) : (
          <span className="text-xs">
            <DataText className="capitalize">{item.planType}</DataText>
            {!hasChildren && (
              <span>
                : <DataText>{item.id}</DataText>
              </span>
            )}
          </span>
        )}
        {item.workerId && (
          <span className="text-xs text-muted-foreground">
            <DataText>Worker: {item.workerId}</DataText>
          </span>
        )}
        {hasChildren && (
          <span className="text-xs text-muted-foreground capitalize text-left">
            <DataText>{`ID: ${item.id}`}</DataText>
          </span>
        )}
      </div>
    );
  };

  return (
    <div className="w-full flex flex-col h-[calc(100vh-4rem)]">
      <div className="flex items-center gap-2 px-4 py-1.5 border-b border-border bg-card flex-shrink-0">
        <Network className="h-4 w-4 text-primary" />
        <h3 className="text-xs font-semibold text-foreground">Query Plan Explorer</h3>
        <div className="text-xs text-muted-foreground">
          <DataText>{queryBundle.entities.query_group.instance_name}</DataText>
          {' - '}
          <DataText>{queryBundle.entities.query.instance_name}</DataText>
        </div>
      </div>

      <ResizablePanelGroup orientation="vertical" className="flex-1">
        <ResizablePanel
          defaultSize="20%"
          minSize="10%"
          collapsible
          collapsedSize="0%"
          className="overflow-y-auto [&::-webkit-scrollbar]:w-0 [scrollbar-width:none] [-ms-overflow-style:none]"
        >
          <TreeView<QueryPlanDataItem>
            data={treeData}
            initialSelectedItemId={planId}
            onSelectChange={handlePlanSelect}
            renderItem={renderItem}
          />
        </ResizablePanel>

        <ResizableHandle withHandle data-panel-group-direction="vertical" />

        {/* DAG Chart - lazy loaded to split elkjs into separate chunk */}
        <ResizablePanel
          defaultSize="75%"
          minSize="25%"
          collapsible
          collapsedSize="0%"
          className="overflow-hidden"
        >
          <Suspense
            fallback={
              <div className="flex items-center justify-center h-full text-muted-foreground">
                Loading visualization...
              </div>
            }
          >
            <DAGChart data={dagData} height="100%" />
          </Suspense>
        </ResizablePanel>
      </ResizablePanelGroup>
    </div>
  );
}
