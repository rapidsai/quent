// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect, lazy, Suspense } from 'react';
import { useQueryBundle } from '@quent/client';
import { useQueryPlanVisualization } from '@/hooks/useQueryPlanVisualization';
import { TreeView } from '@quent/components';
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from '@quent/components';
import { thinScrollbarClass, type QueryPlanDataItem } from '@quent/components';
import { useSelectedPlanId, useSetSelectedPlanId, useSetHoveredWorkerId } from '@quent/hooks';
import { DAGControls, DAGNodeInfoPanel } from '@quent/components';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@quent/components';
import {
  useDagNodeColoring,
  useDagEdgeWidthConfig,
  useDagEdgeColoring,
  useOperatorStatFields,
  usePortStatFields,
} from '@quent/hooks';
import {
  computeNodeColoring,
  computeEdgeWidthConfig,
  computeEdgeColoring,
  parseCustomStatistics,
} from '@quent/components';
import { DataText } from '@quent/components';
import { useTheme, THEME_DARK } from '@/contexts/ThemeContext';

// Lazy load DAGChart to split elkjs (~1.6MB) into a separate chunk
const DAGChart = lazy(() => import('@quent/components').then(mod => ({ default: mod.DAGChart })));

const TABS = {
  PLAN: 'plan',
  CONTROLS: 'controls',
} as const;

export function QueryPlan({ queryId, engineId }: { queryId: string; engineId: string }) {
  const { theme } = useTheme();
  const isDark = theme === THEME_DARK;
  const planId = useSelectedPlanId();
  const setPlanId = useSetSelectedPlanId();
  const setHoveredWorkerId = useSetHoveredWorkerId();

  const {
    data: queryBundle,
    isLoading: queryBundleLoading,
    error: queryBundleError,
  } = useQueryBundle({ engineId, queryId });

  const { dagData, treeData, error: dagError } = useQueryPlanVisualization(queryBundle, planId);

  useDagNodeColoring(dagData.nodes, computeNodeColoring, isDark);
  useDagEdgeWidthConfig(dagData.edges, computeEdgeWidthConfig);
  useDagEdgeColoring(dagData.edges, computeEdgeColoring, isDark);
  const operatorStatFields = useOperatorStatFields(dagData.nodes, parseCustomStatistics);
  const portStatFields = usePortStatFields(dagData.edges);

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
      <div className="flex flex-col items-start py-0.5 pl-1">
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
      <ResizablePanelGroup orientation="vertical" className="flex-1">
        <ResizablePanel defaultSize="15%" className="flex flex-col">
          <Tabs defaultValue={TABS.PLAN}>
            <TabsList>
              <TabsTrigger value={TABS.PLAN}>Query Plan</TabsTrigger>
              <TabsTrigger value={TABS.CONTROLS}>Settings</TabsTrigger>
            </TabsList>
            <TabsContent
              value={TABS.PLAN}
              className={`flex-1 overflow-y-auto ${thinScrollbarClass}`}
            >
              <TreeView<QueryPlanDataItem>
                data={treeData}
                initialSelectedItemId={planId}
                selectedItemId={planId}
                onSelectChange={handlePlanSelect}
                onItemHover={item => setHoveredWorkerId(item?.workerId ?? null)}
                renderItem={renderItem}
              />
            </TabsContent>
            <TabsContent
              value={TABS.CONTROLS}
              className={`flex-1 overflow-y-auto ${thinScrollbarClass}`}
            >
              <DAGControls
                operatorStatFields={operatorStatFields}
                portStatFields={portStatFields}
                isDark={isDark}
              />
            </TabsContent>
          </Tabs>
        </ResizablePanel>

        <ResizableHandle withHandle data-panel-group-direction="vertical" />

        <ResizablePanel
          defaultSize="85%"
          minSize="25%"
          collapsible
          collapsedSize="0%"
          className="overflow-hidden"
        >
          <div className="flex flex-col h-full">
            <div className="flex-1 min-h-0">
              <Suspense
                fallback={
                  <div className="flex items-center justify-center h-full text-muted-foreground">
                    Loading visualization...
                  </div>
                }
              >
                <DAGChart data={dagData} height="100%" isDark={isDark} />
              </Suspense>
            </div>
            <DAGNodeInfoPanel />
          </div>
        </ResizablePanel>
      </ResizablePanelGroup>
    </div>
  );
}
