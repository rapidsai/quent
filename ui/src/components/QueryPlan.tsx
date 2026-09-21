// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  lazy,
  Suspense,
} from 'react';
import type { PanelImperativeHandle } from 'react-resizable-panels';
import { useQueryBundle, useDataFlow } from '@quent/client';
import { useQueryPlanVisualization } from '@/hooks/useQueryPlanVisualization';
import { TreeSelect } from '@quent/components';
import { thinScrollbarClass, type QueryPlanDataItem } from '@quent/components';
import { useSelectedPlanId, useSetSelectedPlanId, useSetHoveredWorkerId } from '@quent/hooks';
import {
  DAGNodeInfoPanel,
  DAGSettingsPopover,
  DagPlayhead,
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from '@quent/components';
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
import { MAX_TIMELINE_BINS, cn } from '@quent/utils';
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

const OPERATOR_DETAILS_COLLAPSED_HEIGHT = 32;
const OPERATOR_DETAILS_DEFAULT_HEIGHT = 224;

export function QueryPlan({ queryId, engineId }: { queryId: string; engineId: string }) {
  const { theme } = useTheme();
  const isDark = theme === THEME_DARK;
  const planId = useSelectedPlanId();
  const setPlanId = useSetSelectedPlanId();
  const setHoveredWorkerId = useSetHoveredWorkerId();
  const [dagSettingsOpen, setDagSettingsOpen] = useState(false);
  const [operatorDetailsExpanded, setOperatorDetailsExpanded] = useState(false);
  const [operatorDetailsPreferredHeight, setOperatorDetailsPreferredHeight] = useState(
    OPERATOR_DETAILS_DEFAULT_HEIGHT
  );
  const [operatorDetailsMaxHeight, setOperatorDetailsMaxHeight] = useState(
    OPERATOR_DETAILS_DEFAULT_HEIGHT
  );
  const operatorDetailsGroupRef = useRef<HTMLDivElement | null>(null);
  const operatorDetailsPanelRef = useRef<PanelImperativeHandle | null>(null);
  const operatorDetailsExpandedHeightRef = useRef(OPERATOR_DETAILS_DEFAULT_HEIGHT);
  const {
    data: queryBundle,
    isLoading: queryBundleLoading,
    error: queryBundleError,
  } = useQueryBundle({ engineId, queryId });

  const { dagData, treeData, error: dagError } = useQueryPlanVisualization(queryBundle, planId);
  const operators = useMemo(
    () => Object.values(queryBundle?.entities.operators ?? {}),
    [queryBundle?.entities.operators]
  );

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

  const handlePlanSelect = (item: QueryPlanDataItem) => setPlanId(item.id);
  const closeDagSettings = useCallback(() => setDagSettingsOpen(false), []);

  useEffect(() => {
    if (queryBundle && !planId) {
      setPlanId(queryBundle.plan_tree.id);
    }
  }, [queryBundle, planId, setPlanId]);

  useEffect(() => {
    const panel = operatorDetailsPanelRef.current;
    if (operatorDetailsExpanded) {
      panel?.expand();
      panel?.resize(Math.min(operatorDetailsExpandedHeightRef.current, operatorDetailsMaxHeight));
    } else {
      panel?.collapse();
    }
  }, [operatorDetailsExpanded, operatorDetailsMaxHeight]);

  useLayoutEffect(() => {
    const group = operatorDetailsGroupRef.current;
    if (!group) {
      return;
    }
    const updateMaxHeight = () => {
      if (group.clientHeight === 0) {
        return;
      }
      setOperatorDetailsMaxHeight(
        Math.max(96, Math.min(operatorDetailsPreferredHeight, group.clientHeight * 0.5))
      );
    };
    updateMaxHeight();
    if (typeof ResizeObserver === 'undefined') {
      return;
    }
    const observer = new ResizeObserver(updateMaxHeight);
    observer.observe(group);
    return () => observer.disconnect();
  }, [operatorDetailsPreferredHeight]);

  useEffect(() => {
    const panel = operatorDetailsPanelRef.current;
    if (operatorDetailsExpanded && panel && panel.getSize().inPixels > operatorDetailsMaxHeight) {
      panel.resize(operatorDetailsMaxHeight);
    }
  }, [operatorDetailsExpanded, operatorDetailsMaxHeight]);

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

  const getPlanItemTitle = (item: QueryPlanDataItem) => {
    const hasChildren = !!item.children?.length;
    const primary = singleQueryPlan
      ? `Query: ${item.queryId ?? item.id}`
      : hasChildren
        ? (item.planType ?? item.name)
        : [item.planType, item.id].filter(Boolean).join(': ');
    const lines = [primary];
    if (item.workerId) {
      lines.push(`Worker: ${item.workerId}`);
    }
    if (hasChildren) {
      lines.push(`ID: ${item.id}`);
    }
    return lines.join('\n');
  };

  const renderPlanItem = (item: QueryPlanDataItem, hasChildren: boolean, compact = false) => {
    return (
      <div
        className={cn('flex w-full min-w-0 flex-col items-start overflow-hidden pl-1', {
          'py-0.5': !compact,
        })}
      >
        {singleQueryPlan ? (
          <span className="block w-full truncate text-xs">
            Query: <DataText>{item.queryId}</DataText>
          </span>
        ) : (
          <span className="block w-full truncate text-xs">
            <DataText className="capitalize">{item.planType}</DataText>
            {!hasChildren && (
              <span>
                : <DataText>{item.id}</DataText>
              </span>
            )}
          </span>
        )}
        {item.workerId && (
          <span className="block w-full truncate text-xs text-muted-foreground">
            <DataText>Worker: {item.workerId}</DataText>
          </span>
        )}
        {hasChildren && (
          <span className="block w-full truncate text-left text-xs text-muted-foreground capitalize">
            <DataText>{`ID: ${item.id}`}</DataText>
          </span>
        )}
      </div>
    );
  };

  return (
    <div className="w-full flex flex-col h-[calc(100vh-4rem)]">
      {/* my-2px lines it up with timeline rows */}
      <section className="my-[2px] flex min-w-0 shrink-0 items-center gap-1.5 overflow-hidden border-b px-1.5 py-2.5">
        <TreeSelect<QueryPlanDataItem>
          data={treeData}
          value={planId}
          onValueChange={handlePlanSelect}
          onItemHover={item => setHoveredWorkerId(item?.workerId ?? null)}
          ariaLabel="Query plan"
          title="Query Plan"
          getItemTitle={getPlanItemTitle}
          collapsible={false}
          renderItem={({ item, hasChildren }) => renderPlanItem(item, hasChildren, true)}
          renderValue={item => renderPlanItem(item, !!item.children?.length)}
          triggerClassName="overflow-hidden"
          contentClassName={`min-w-64 ${thinScrollbarClass}`}
        />
        <DAGSettingsPopover
          operatorStatFields={operatorStatFields}
          portStatFields={portStatFields}
          isDark={isDark}
          open={dagSettingsOpen}
          onOpenChange={setDagSettingsOpen}
        />
      </section>

      <ResizablePanelGroup
        orientation="vertical"
        className="min-h-0 flex-1"
        elementRef={operatorDetailsGroupRef}
      >
        <ResizablePanel id="query-plan-dag" minSize="25%">
          <div className="flex h-full min-h-0 flex-col overflow-hidden">
            <div className="flex-1 min-h-0">
              <Suspense
                fallback={
                  <div className="flex items-center justify-center h-full text-muted-foreground">
                    Loading visualization...
                  </div>
                }
              >
                <DAGChart
                  data={dagData}
                  height="100%"
                  isDark={isDark}
                  operators={operators}
                  onBackgroundClick={closeDagSettings}
                />
              </Suspense>
            </div>
            <DagPlayhead />
          </div>
        </ResizablePanel>
        <ResizableHandle
          withHandle={operatorDetailsExpanded}
          disabled={!operatorDetailsExpanded}
          className={operatorDetailsExpanded ? undefined : 'opacity-0'}
        />
        <ResizablePanel
          id="operator-details"
          panelRef={operatorDetailsPanelRef}
          defaultSize={OPERATOR_DETAILS_COLLAPSED_HEIGHT}
          minSize={96}
          maxSize={operatorDetailsMaxHeight}
          collapsible
          collapsedSize={OPERATOR_DETAILS_COLLAPSED_HEIGHT}
          groupResizeBehavior="preserve-pixel-size"
          className="min-h-0 overflow-hidden"
          onResize={size => {
            if (operatorDetailsExpanded && size.inPixels > OPERATOR_DETAILS_COLLAPSED_HEIGHT) {
              operatorDetailsExpandedHeightRef.current = size.inPixels;
            }
          }}
        >
          <DAGNodeInfoPanel
            isDark={isDark}
            quantitySpecs={queryBundle.quantity_specs}
            fillHeight
            onExpandedChange={setOperatorDetailsExpanded}
            onPreferredHeightChange={setOperatorDetailsPreferredHeight}
          />
        </ResizablePanel>
      </ResizablePanelGroup>
    </div>
  );
}
