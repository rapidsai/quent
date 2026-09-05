// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useLayoutEffect, useMemo, useRef, lazy, Suspense } from 'react';
import type { PanelImperativeHandle } from 'react-resizable-panels';
import { useQueryBundle, useDataFlow } from '@quent/client';
import { useQueryPlanVisualization } from '@/hooks/useQueryPlanVisualization';
import { Badge, getSelectedOperatorCountsByPlan, TreeView } from '@quent/components';
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from '@quent/components';
import { thinScrollbarClass, type QueryPlanDataItem } from '@quent/components';
import {
  useSelectedNodeIds,
  useSelectedPlanId,
  useSetSelectedPlanId,
  useSetHoveredWorkerId,
} from '@quent/hooks';
import { DAGControls, DAGNodeInfoPanel, DagPlayhead } from '@quent/components';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@quent/components';
import {
  useDagNodeColoring,
  useDagEdgeWidthConfig,
  useDagEdgeColoring,
  useOperatorStatFields,
  usePortStatFields,
  useDataFlowSync,
  useDebouncedZoomRange,
  resolveDataFlowWindow,
} from '@quent/hooks';
import { MAX_TIMELINE_BINS } from '@quent/utils';
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

const MAX_TOP_PANEL_HEIGHT_PX = 300;

export function QueryPlan({ queryId, engineId }: { queryId: string; engineId: string }) {
  const { theme } = useTheme();
  const isDark = theme === THEME_DARK;
  const planId = useSelectedPlanId();
  const setPlanId = useSetSelectedPlanId();
  const setHoveredWorkerId = useSetHoveredWorkerId();
  const selectedOperatorIds = useSelectedNodeIds();
  const {
    data: queryBundle,
    isLoading: queryBundleLoading,
    error: queryBundleError,
  } = useQueryBundle({ engineId, queryId });
  const selectedOperatorCountsByPlan = useMemo(
    () =>
      queryBundle
        ? getSelectedOperatorCountsByPlan(queryBundle, selectedOperatorIds)
        : new Map<string, number>(),
    [queryBundle, selectedOperatorIds]
  );

  const { dagData, treeData, error: dagError } = useQueryPlanVisualization(queryBundle, planId);

  // Data-flow overlay: fetch the categorical timeline for the current zoom
  // window (fallback: full query duration) and sync it into the data-flow
  // atoms. The first response doubles as the feature probe — `null` (HTTP
  // 501, analyzer without data-flow support) or an empty result hides the
  // playhead, bars, controls, and legend entries.
  const debouncedZoomRange = useDebouncedZoomRange();
  const dataFlowWindow = resolveDataFlowWindow(debouncedZoomRange, queryBundle?.duration_s ?? 0);
  const { data: dataFlowResponse } = useDataFlow(
    {
      engineId,
      queryId,
      config: {
        num_bins: MAX_TIMELINE_BINS,
        start: dataFlowWindow.start,
        end: dataFlowWindow.end,
      },
    },
    { enabled: !!queryBundle && dataFlowWindow.end > dataFlowWindow.start }
  );
  useDataFlowSync({ response: dataFlowResponse, queryBundle });

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

  const topPanelRef = useRef<PanelImperativeHandle | null>(null);
  const treeContentRef = useRef<HTMLDivElement>(null);
  const tabsListRef = useRef<HTMLDivElement>(null);

  // Resize the top panel to fit tree content (capped at MAX_TOP_PANEL_HEIGHT_PX).
  // Note: PanelImperativeHandle.resize() treats numbers as pixels.
  useLayoutEffect(() => {
    const treeContent = treeContentRef.current;
    const topPanel = topPanelRef.current;
    if (!treeContent || !topPanel) {
      return;
    }

    const tabsListHeight = tabsListRef.current?.offsetHeight ?? 0;
    const desiredPx = treeContent.scrollHeight + tabsListHeight;
    const cappedPx = Math.min(desiredPx, MAX_TOP_PANEL_HEIGHT_PX);
    topPanel.resize(cappedPx);
  }, [treeData, planId]);

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
    const selectedOperatorCount = selectedOperatorCountsByPlan.get(item.id) ?? 0;
    const selectedOperatorLabel = `${selectedOperatorCount} selected operator${
      selectedOperatorCount === 1 ? '' : 's'
    } in this plan`;

    return (
      <div className="flex flex-col items-start py-0.5 pl-1">
        <div className="flex items-center gap-1.5">
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
          {selectedOperatorCount > 0 && (
            <Badge
              variant="secondary"
              className="h-4 min-w-4 rounded-full px-1 py-0 text-[10px] leading-none"
              aria-label={selectedOperatorLabel}
              title={selectedOperatorLabel}
            >
              {selectedOperatorCount}
            </Badge>
          )}
        </div>
        {item.workerId && (
          <span className="text-xs text-muted-foreground">
            <DataText>Worker: {item.workerName ?? item.workerId}</DataText>
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
        <ResizablePanel
          panelRef={topPanelRef}
          defaultSize="15%"
          minSize={80}
          maxSize={MAX_TOP_PANEL_HEIGHT_PX}
          className="flex flex-col"
        >
          <Tabs defaultValue={TABS.PLAN}>
            <TabsList ref={tabsListRef}>
              <TabsTrigger value={TABS.PLAN}>Query Plan</TabsTrigger>
              <TabsTrigger value={TABS.CONTROLS}>Settings</TabsTrigger>
            </TabsList>
            <TabsContent
              value={TABS.PLAN}
              className={`flex-1 overflow-y-auto ${thinScrollbarClass}`}
            >
              <div ref={treeContentRef}>
                <TreeView<QueryPlanDataItem>
                  data={treeData}
                  initialSelectedItemId={planId}
                  selectedItemId={planId}
                  onSelectChange={handlePlanSelect}
                  onItemHover={item => setHoveredWorkerId(item?.workerId ?? null)}
                  renderItem={renderItem}
                />
              </div>
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
            <DagPlayhead />
            <DAGNodeInfoPanel isDark={isDark} quantitySpecs={queryBundle.quantity_specs} />
          </div>
        </ResizablePanel>
      </ResizablePanelGroup>
    </div>
  );
}
