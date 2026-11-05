// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useLayoutEffect, useRef, useState, type ReactNode } from 'react';
import { ChevronDown, ChevronUp } from 'lucide-react';
import {
  useDataFlowEnabled,
  useDataFlowFrame,
  useDataFlowIsPlaying,
  useDataFlowMeta,
  useClearDagDetailAttention,
  useDagDetailAttention,
  useDisplayedOperatorDetails,
  useInspectedPipes,
  useInspectedPipeNodes,
  useOperatorSelectionActions,
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

function scrollToDetailEntry(entry: HTMLElement) {
  const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  entry.scrollIntoView({
    behavior: reduceMotion ? 'auto' : 'smooth',
    block: 'nearest',
  });
}

function DetailAttentionEntry({
  kind,
  id,
  testId,
  className,
  children,
}: {
  kind: 'operator' | 'pipe';
  id: string;
  testId: string;
  className?: string;
  children: ReactNode;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const attention = useDagDetailAttention();
  const clearAttention = useClearDagDetailAttention();

  useLayoutEffect(() => {
    if (attention?.activateTab && attention.kind === kind && attention.id === id && ref.current) {
      scrollToDetailEntry(ref.current);
      clearAttention(attention);
    }
  }, [attention, clearAttention, id, kind]);

  return (
    <div ref={ref} data-testid={testId} className={className}>
      {children}
    </div>
  );
}

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
  const inspectedPipes = useInspectedPipes();
  const inspectedPipeNodes = useInspectedPipeNodes();
  const updateOperatorSelection = useOperatorSelectionActions();
  const toggleInspectedPipe = useToggleInspectedPipe();
  const selectedOperators = operatorDetails.map(detail => detail.data);
  const dataFlowEnabled = useDataFlowEnabled();
  const isPlaying = useDataFlowIsPlaying();
  const dataFlowMeta = useDataFlowMeta();
  const dataFlowFrame = useDataFlowFrame();
  const detailAttention = useDagDetailAttention();
  const clearDetailAttention = useClearDagDetailAttention();
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
  const [prevSelectedOperatorIdsKey, setPrevSelectedOperatorIdsKey] =
    useState(selectedOperatorIdsKey);
  if (selectedOperatorIdsKey !== prevSelectedOperatorIdsKey) {
    setPrevSelectedOperatorIdsKey(selectedOperatorIdsKey);
    setClosedOperatorIds(new Set());
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

  useLayoutEffect(() => {
    if (!detailAttention) {
      return;
    }
    if (!detailAttention.activateTab) {
      clearDetailAttention(detailAttention);
      return;
    }
    setIsExpanded(true);
    setActiveTab(detailAttention.kind === 'pipe' ? 'pipes' : 'stats');
  }, [clearDetailAttention, detailAttention]);

  const scrollClass = cn(
    'px-4 pb-2 overflow-auto',
    fillHeight ? 'min-h-0 flex-1' : 'h-48',
    thinScrollbarClass
  );

  const statsContent = showStatsTab ? (
    <div ref={statsContentRef} className="flex flex-col gap-1 pt-1.5">
      {operatorDetails.map((detail, index) => (
        <DetailAttentionEntry
          key={detail.selectionId}
          kind="operator"
          id={detail.selectionId}
          testId={`operator-detail-entry-${detail.selectionId}`}
          className={cn(index > 0 && 'border-t pt-1.5 mt-1.5')}
        >
          <OperatorDetailsBlock
            operator={detail.data}
            quantitySpecs={quantitySpecs}
            isOpen={isOperatorOpen}
            onOpenChange={setOperatorOpen}
            actions={
              <DetailItemActions
                label={`operator ${detail.data.label}`}
                onRemove={() =>
                  updateOperatorSelection({
                    type: 'remove',
                    selectionId: detail.selectionId,
                  })
                }
                title={'Remove filter'}
              />
            }
          />
        </DetailAttentionEntry>
      ))}
    </div>
  ) : null;

  const pipesContent = showPipesTab ? (
    <div ref={pipesContentRef} className="flex flex-col gap-1 pt-1.5">
      {inspectedPipes.map((pipe, index) => (
        <DetailAttentionEntry
          key={pipe.id}
          kind="pipe"
          id={pipe.id}
          testId={`pipe-detail-entry-${pipe.id}`}
          className={cn(index > 0 && 'border-t pt-3 mt-3')}
        >
          <PipeDetailsBlock
            pipe={pipe}
            open={isOperatorOpen(pipe.id)}
            onOpenChange={open => setOperatorOpen(pipe.id, open)}
            actions={
              <DetailItemActions
                label={`pipe from ${inspectedPipeNodes.get(pipe.source)?.label ?? pipe.source} to ${inspectedPipeNodes.get(pipe.target)?.label ?? pipe.target}`}
                onRemove={() => {
                  if (pipe.sourcePortId && pipe.targetPortId) {
                    toggleInspectedPipe({
                      sourcePortId: pipe.sourcePortId,
                      targetPortId: pipe.targetPortId,
                    });
                  }
                }}
              />
            }
          />
        </DetailAttentionEntry>
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
