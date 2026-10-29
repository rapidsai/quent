// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useLayoutEffect, useRef, useState } from 'react';
import { ChevronDown, ChevronUp } from 'lucide-react';
import {
  useDataFlowEnabled,
  useDataFlowFrame,
  useDataFlowIsPlaying,
  useDataFlowMeta,
  useDisplayedOperatorDetails,
  useDisplayedPipeDetails,
  useInspectedPipeNodes,
  useOperatorDetailsActions,
  useOperatorSelection,
  useOperatorSelectionActions,
  usePinnedOperatorDetailIds,
  useSelectedOperatorIds,
  useToggleInspectedPipe,
} from '@quent/hooks';
import { cn, type QuantitySpec } from '@quent/utils';
import {
  DetailItemActions,
  OperatorDataFlowBlock,
  OperatorDetailsBlock,
  PipeDetailsBlock,
} from '../node-info';
import { thinScrollbarClass } from '../ui/thin-scroll';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../ui/tabs';

type DetailsTab = 'stats' | 'pipes' | 'data-flow';

export const DAGNodeInfoPanel = ({
  quantitySpecs,
  fillHeight = false,
  onExpandedChange,
  onPreferredHeightChange,
}: {
  quantitySpecs?: { [key: string]: QuantitySpec | undefined };
  fillHeight?: boolean;
  onExpandedChange?: (expanded: boolean) => void;
  onPreferredHeightChange?: (height: number) => void;
}) => {
  const operatorDetails = useDisplayedOperatorDetails();
  const pipeDetails = useDisplayedPipeDetails();
  const inspectedPipeNodes = useInspectedPipeNodes();
  const operatorSelection = useOperatorSelection();
  const updateOperatorSelection = useOperatorSelectionActions();
  const selectedOperatorIds = useSelectedOperatorIds();
  const pinnedOperatorDetailIds = usePinnedOperatorDetailIds();
  const updateOperatorDetails = useOperatorDetailsActions();
  const toggleInspectedPipe = useToggleInspectedPipe();
  const selectedOperators = operatorDetails.map(detail => detail.data);
  const inspectedPipes = pipeDetails.map(detail => detail.pipe);
  const dataFlowEnabled = useDataFlowEnabled();
  const isPlaying = useDataFlowIsPlaying();
  const dataFlowMeta = useDataFlowMeta();
  const dataFlowFrame = useDataFlowFrame();
  const [isExpanded, setIsExpanded] = useState(
    inspectedPipes.length > 0 || selectedOperators.length > 0
  );
  const [requestedTab, setActiveTab] = useState<DetailsTab>('stats');
  const [closedOperatorIds, setClosedOperatorIds] = useState<Set<string>>(() => new Set());
  const headerRef = useRef<HTMLDivElement>(null);
  const tabsListRef = useRef<HTMLDivElement>(null);
  const statsContentRef = useRef<HTMLDivElement>(null);
  const pipesContentRef = useRef<HTMLDivElement>(null);
  const dataFlowContentRef = useRef<HTMLDivElement>(null);
  const hasSelection = inspectedPipes.length > 0 || selectedOperators.length > 0;
  const selectedOperatorIdsKey = selectedOperators.map(operator => operator.nodeId).join('\0');
  const inspectedPipeIdsKey = inspectedPipes.map(pipe => pipe.id).join('\0');
  // Notify the containing resizable panel after committing local expansion state.
  useLayoutEffect(() => {
    onExpandedChange?.(isExpanded);
  }, [isExpanded, onExpandedChange]);

  const showStatsTab = selectedOperators.length > 0;
  const showPipesTab = inspectedPipes.length > 0;
  const showDataFlowTab = showStatsTab && dataFlowEnabled && dataFlowMeta != null;
  const availableTabs: DetailsTab[] = [
    ...(showStatsTab ? (['stats'] as const) : []),
    ...(showPipesTab ? (['pipes'] as const) : []),
    ...(showDataFlowTab ? (['data-flow'] as const) : []),
  ];
  // A tab can disappear when its selection is cleared; fall back to the first remaining one.
  const activeTab = availableTabs.includes(requestedTab) ? requestedTab : availableTabs[0];
  const hasTabs = availableTabs.length > 1;
  const isOperatorOpen = (id: string) => !closedOperatorIds.has(id);
  const setOperatorOpen = (id: string, open: boolean) => {
    setClosedOperatorIds(prev => {
      const isClosed = prev.has(id);
      if (open ? !isClosed : isClosed) {
        return prev;
      }
      const next = new Set(prev);
      if (open) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  };

  // Expand/collapse and reset the tab as soon as the selection is made or cleared.
  // Uses the prev-state-in-render pattern instead of useEffect to avoid a visible flicker.
  const [prevHasSelection, setPrevHasSelection] = useState(hasSelection);
  if (hasSelection !== prevHasSelection) {
    setPrevHasSelection(hasSelection);
    setIsExpanded(hasSelection);
    if (!hasSelection) {
      setActiveTab('stats');
    }
  }

  // Collapse per-operator sections as soon as the selected operators change.
  // Selecting one more operator reveals the Operators tab.
  const [prevSelectedOperatorIdsKey, setPrevSelectedOperatorIdsKey] =
    useState(selectedOperatorIdsKey);
  const [prevOperatorCount, setPrevOperatorCount] = useState(selectedOperators.length);
  if (selectedOperatorIdsKey !== prevSelectedOperatorIdsKey) {
    setPrevSelectedOperatorIdsKey(selectedOperatorIdsKey);
    setPrevOperatorCount(selectedOperators.length);
    setClosedOperatorIds(new Set());
    if (selectedOperators.length > prevOperatorCount) {
      setActiveTab('stats');
      setIsExpanded(true);
    }
  }

  // Selecting one more pipe reveals the Pipes tab and expands the panel.
  const [prevInspectedPipeIdsKey, setPrevInspectedPipeIdsKey] = useState(inspectedPipeIdsKey);
  const [prevPipeCount, setPrevPipeCount] = useState(inspectedPipes.length);
  if (inspectedPipeIdsKey !== prevInspectedPipeIdsKey) {
    setPrevInspectedPipeIdsKey(inspectedPipeIdsKey);
    setPrevPipeCount(inspectedPipes.length);
    if (inspectedPipes.length > prevPipeCount) {
      setActiveTab('pipes');
      setIsExpanded(true);
    }
  }

  // Jump to the data-flow tab as soon as it becomes available during playback
  const shouldShowDataFlowTab = isPlaying && isExpanded && showDataFlowTab;
  const [prevShouldShowDataFlowTab, setPrevShouldShowDataFlowTab] = useState(shouldShowDataFlowTab);
  if (shouldShowDataFlowTab !== prevShouldShowDataFlowTab) {
    setPrevShouldShowDataFlowTab(shouldShowDataFlowTab);
    if (shouldShowDataFlowTab) {
      setActiveTab('data-flow');
    }
  }

  const scrollClass = cn(
    'px-4 pb-2 overflow-auto',
    fillHeight ? 'min-h-0 flex-1' : 'h-48',
    thinScrollbarClass
  );

  const statsContent = showStatsTab ? (
    <div ref={statsContentRef} className="flex flex-col gap-1 pt-1.5">
      {operatorDetails.map((detail, index) => {
        const operatorIds = new Set([
          detail.data.nodeId,
          ...(detail.data.relatedOperators?.map(operator => operator.nodeId) ?? []),
        ]);
        const filtered = [...operatorIds].every(id => selectedOperatorIds.has(id));
        const matchingFilterIds = [...operatorSelection.selections]
          .filter(
            ([selectionId, selection]) =>
              selectionId === detail.selectionId ||
              (selection.operatorIds.size === operatorIds.size &&
                [...selection.operatorIds].every(id => operatorIds.has(id)))
          )
          .map(([selectionId]) => selectionId);
        const pinned = pinnedOperatorDetailIds.has(detail.selectionId);
        const toggleFilter = () => {
          if (filtered) {
            for (const selectionId of matchingFilterIds) {
              updateOperatorSelection({ type: 'remove', selectionId });
            }
          } else {
            updateOperatorSelection({
              type: 'add',
              selectionId: detail.selectionId,
              label: detail.data.label,
              operatorIds,
              selectedData: detail.data,
            });
          }
        };

        return (
          <div key={detail.selectionId} className={index > 0 ? 'border-t pt-1.5 mt-1.5' : ''}>
            <OperatorDetailsBlock
              operator={detail.data}
              quantitySpecs={quantitySpecs}
              isOpen={isOperatorOpen}
              onOpenChange={setOperatorOpen}
              actions={
                <DetailItemActions
                  label={`operator ${detail.data.label}`}
                  filtered={filtered}
                  pinned={pinned}
                  onToggleFilter={toggleFilter}
                  onTogglePin={() =>
                    updateOperatorDetails(
                      pinned
                        ? { type: 'unpin', selectionId: detail.selectionId }
                        : {
                            type: 'pin',
                            entry: { selectionId: detail.selectionId, data: detail.data },
                          }
                    )
                  }
                />
              }
            />
          </div>
        );
      })}
    </div>
  ) : null;

  const pipesContent = showPipesTab ? (
    <div ref={pipesContentRef} className="flex flex-col gap-1 pt-1.5">
      {pipeDetails.map((detail, index) => (
        <div key={detail.pipe.id} className={index > 0 ? 'border-t pt-3 mt-3' : ''}>
          <PipeDetailsBlock
            pipe={detail.pipe}
            open={isOperatorOpen(detail.pipe.id)}
            onOpenChange={open => setOperatorOpen(detail.pipe.id, open)}
            actions={
              <DetailItemActions
                label={`pipe from ${inspectedPipeNodes.get(detail.pipe.source)?.label ?? detail.pipe.source} to ${inspectedPipeNodes.get(detail.pipe.target)?.label ?? detail.pipe.target}`}
                pinned
                onUnpin={() => {
                  if (detail.pipe.sourcePortId && detail.pipe.targetPortId) {
                    toggleInspectedPipe({
                      sourcePortId: detail.pipe.sourcePortId,
                      targetPortId: detail.pipe.targetPortId,
                    });
                  }
                }}
              />
            }
          />
        </div>
      ))}
    </div>
  ) : null;

  const dataFlowContent =
    dataFlowMeta && dataFlowFrame ? (
      <div ref={dataFlowContentRef} className="flex flex-col">
        {selectedOperators.map((operator, index) => (
          <div key={operator.nodeId} className={index > 0 ? 'border-t pt-1.5 mt-1.5' : ''}>
            <OperatorDataFlowBlock
              operator={operator}
              meta={dataFlowMeta}
              frame={dataFlowFrame}
              isOpen={isOperatorOpen}
              onOpenChange={setOperatorOpen}
            />
          </div>
        ))}
      </div>
    ) : (
      <div ref={dataFlowContentRef}>
        <p className="pt-6 text-xs text-muted-foreground text-center">No tasks at this bin</p>
      </div>
    );

  const activeContentRef = useCallback(() => {
    switch (activeTab) {
      case 'pipes':
        return pipesContentRef.current;
      case 'data-flow':
        return dataFlowContentRef.current;
      default:
        return statsContentRef.current;
    }
  }, [activeTab]);

  const measurePreferredHeight = useCallback(() => {
    if (!onPreferredHeightChange || !isExpanded) {
      return;
    }
    const content = activeContentRef();
    const headerHeight = headerRef.current?.offsetHeight ?? 0;
    const tabsHeight = hasTabs ? (tabsListRef.current?.offsetHeight ?? 0) : 0;
    const contentHeight = content?.scrollHeight ?? 0;
    if (headerHeight === 0 || contentHeight === 0 || (hasTabs && tabsHeight === 0)) {
      return;
    }
    const height = headerHeight + tabsHeight + contentHeight + 10;
    onPreferredHeightChange(height);
  }, [activeContentRef, hasTabs, isExpanded, onPreferredHeightChange]);

  useLayoutEffect(() => {
    if (!onPreferredHeightChange) {
      return;
    }
    measurePreferredHeight();
    const content = activeContentRef();
    if (!content || typeof ResizeObserver === 'undefined') {
      return;
    }
    const observer = new ResizeObserver(measurePreferredHeight);
    observer.observe(content);
    return () => observer.disconnect();
  }, [
    activeContentRef,
    closedOperatorIds,
    dataFlowFrame,
    inspectedPipeIdsKey,
    measurePreferredHeight,
    onPreferredHeightChange,
    selectedOperatorIdsKey,
  ]);

  return (
    <div
      className={cn(
        'border-t bg-card flex-shrink-0',
        fillHeight && 'flex h-full min-h-0 flex-col overflow-hidden'
      )}
    >
      <div
        ref={headerRef}
        className="flex shrink-0 items-center justify-between px-4 py-1.5 min-w-0"
      >
        <div className="flex items-center gap-2 min-w-0 overflow-hidden">
          <span className="text-xs text-muted-foreground font-medium flex-shrink-0">Details</span>
        </div>
        <div className="ml-2 flex flex-shrink-0 items-center gap-1">
          <button
            onClick={() => setIsExpanded(!isExpanded)}
            disabled={!hasSelection}
            className="rounded p-1 hover:bg-muted transition-colors cursor-pointer disabled:opacity-40 disabled:cursor-auto disabled:hover:bg-transparent flex-shrink-0"
            aria-label="Toggle details"
          >
            {isExpanded ? (
              <ChevronDown className="h-3 w-3 text-muted-foreground" />
            ) : (
              <ChevronUp className="h-3 w-3 text-muted-foreground" />
            )}
          </button>
        </div>
      </div>

      {isExpanded &&
        hasSelection &&
        (hasTabs ? (
          <Tabs
            value={activeTab}
            onValueChange={value => setActiveTab(value as DetailsTab)}
            className={cn(
              'border-t',
              fillHeight ? 'min-h-0 flex-1 overflow-hidden' : 'overflow-visible'
            )}
          >
            <TabsList ref={tabsListRef} className="h-7 py-0 px-1 rounded-none">
              {showStatsTab && (
                <TabsTrigger value="stats" className="text-xs px-2 py-0.5">
                  Operators
                </TabsTrigger>
              )}
              {showPipesTab && (
                <TabsTrigger value="pipes" className="text-xs px-2 py-0.5">
                  Pipes
                </TabsTrigger>
              )}
              {showDataFlowTab && (
                <TabsTrigger value="data-flow" className="text-xs px-2 py-0.5">
                  Data Flow
                </TabsTrigger>
              )}
            </TabsList>
            {showStatsTab && (
              <TabsContent value="stats" className={scrollClass}>
                {statsContent}
              </TabsContent>
            )}
            {showPipesTab && (
              <TabsContent value="pipes" className={scrollClass}>
                {pipesContent}
              </TabsContent>
            )}
            {showDataFlowTab && (
              <TabsContent value="data-flow" className={scrollClass}>
                {dataFlowContent}
              </TabsContent>
            )}
          </Tabs>
        ) : (
          <div className={cn('border-t', scrollClass)}>{statsContent ?? pipesContent}</div>
        ))}
    </div>
  );
};
