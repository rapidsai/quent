// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useLayoutEffect, useRef, useState } from 'react';
import { ChevronDown, ChevronUp } from 'lucide-react';
import {
  useDataFlowEnabled,
  useDataFlowFrame,
  useDataFlowIsPlaying,
  useDataFlowMeta,
  useSelectedOperatorsData,
} from '@quent/hooks';
import { cn, type QuantitySpec } from '@quent/utils';
import { OperatorColorBar, OperatorDataFlowBlock, OperatorDetailsBlock } from '../node-info';
import { DataText } from '../ui/data-text';
import { thinScrollbarClass } from '../ui/thin-scroll';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../ui/tabs';

export const DAGNodeInfoPanel = ({
  isDark = false,
  quantitySpecs,
  fillHeight = false,
  onExpandedChange,
  onPreferredHeightChange,
}: {
  isDark?: boolean;
  quantitySpecs?: { [key: string]: QuantitySpec | undefined };
  fillHeight?: boolean;
  onExpandedChange?: (expanded: boolean) => void;
  onPreferredHeightChange?: (height: number) => void;
}) => {
  const selectedOperators = useSelectedOperatorsData();
  const dataFlowEnabled = useDataFlowEnabled();
  const isPlaying = useDataFlowIsPlaying();
  const dataFlowMeta = useDataFlowMeta();
  const dataFlowFrame = useDataFlowFrame();
  const [isExpanded, setIsExpanded] = useState(false);
  const [activeTab, setActiveTab] = useState('stats');
  const [closedOperatorIds, setClosedOperatorIds] = useState<Set<string>>(() => new Set());
  const headerRef = useRef<HTMLDivElement>(null);
  const tabsListRef = useRef<HTMLDivElement>(null);
  const statsContentRef = useRef<HTMLDivElement>(null);
  const dataFlowContentRef = useRef<HTMLDivElement>(null);
  const hasSelection = selectedOperators.length > 0;
  const showHeaders = selectedOperators.length > 1;
  const selectedOperator = selectedOperators[0];
  const selectedOperatorIdsKey = selectedOperators.map(operator => operator.nodeId).join('\0');
  const updateExpanded = useCallback(
    (expanded: boolean) => {
      setIsExpanded(expanded);
      onExpandedChange?.(expanded);
    },
    [onExpandedChange]
  );

  const showDataFlowTab = dataFlowEnabled && dataFlowMeta != null;
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
    updateExpanded(hasSelection);
    if (!hasSelection) {
      setActiveTab('stats');
    }
  }

  // Collapse per-operator sections as soon as the selected operators change
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

  const scrollClass = cn(
    'px-4 pb-2 overflow-auto',
    fillHeight ? 'min-h-0 flex-1' : 'h-48',
    thinScrollbarClass
  );

  const statsContent = hasSelection ? (
    <div ref={statsContentRef} className="flex flex-col gap-1 pr-2 pt-1.5">
      {selectedOperators.map((operator, index) => (
        <div key={operator.nodeId} className={index > 0 ? 'border-t pt-1.5 mt-1.5' : ''}>
          <OperatorDetailsBlock
            operator={operator}
            quantitySpecs={quantitySpecs}
            isOpen={isOperatorOpen}
            onOpenChange={setOperatorOpen}
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
              isDark={isDark}
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

  const measurePreferredHeight = useCallback(() => {
    if (!onPreferredHeightChange || !isExpanded) {
      return;
    }
    const content =
      showDataFlowTab && activeTab === 'data-flow'
        ? dataFlowContentRef.current
        : statsContentRef.current;
    const headerHeight = headerRef.current?.offsetHeight ?? 0;
    const tabsHeight = showDataFlowTab ? (tabsListRef.current?.offsetHeight ?? 0) : 0;
    const contentHeight = content?.scrollHeight ?? 0;
    if (headerHeight === 0 || contentHeight === 0 || (showDataFlowTab && tabsHeight === 0)) {
      return;
    }
    const height = headerHeight + tabsHeight + contentHeight + 10;
    onPreferredHeightChange(height);
  }, [activeTab, isExpanded, onPreferredHeightChange, showDataFlowTab]);

  useLayoutEffect(() => {
    if (!onPreferredHeightChange) {
      return;
    }
    measurePreferredHeight();
    const content =
      showDataFlowTab && activeTab === 'data-flow'
        ? dataFlowContentRef.current
        : statsContentRef.current;
    if (!content || typeof ResizeObserver === 'undefined') {
      return;
    }
    const observer = new ResizeObserver(measurePreferredHeight);
    observer.observe(content);
    return () => observer.disconnect();
  }, [
    activeTab,
    closedOperatorIds,
    dataFlowFrame,
    measurePreferredHeight,
    onPreferredHeightChange,
    selectedOperatorIdsKey,
    showDataFlowTab,
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
          <span className="text-xs text-muted-foreground font-medium flex-shrink-0">
            Operator Details
          </span>
          {selectedOperator && (
            <>
              <span className="text-muted-foreground text-xs flex-shrink-0">·</span>
              <div
                data-testid="operator-details-title"
                className="flex min-w-0 items-center gap-1.5 overflow-hidden"
              >
                {selectedOperators.map((operator, index) => (
                  <span key={operator.nodeId} className="flex min-w-0 items-center gap-1">
                    {index > 0 && <span className="text-muted-foreground text-xs shrink-0">,</span>}
                    <OperatorColorBar operationType={operator.operationType} className="h-3 w-1" />
                    <DataText className="text-xs font-medium truncate" title={operator.label}>
                      {operator.label}
                    </DataText>
                    {!showHeaders && (
                      <DataText className="text-xs text-muted-foreground capitalize px-1.5 py-0.5 bg-muted rounded flex-shrink-0">
                        {operator.operationType}
                      </DataText>
                    )}
                  </span>
                ))}
              </div>
            </>
          )}
        </div>
        <button
          onClick={() => updateExpanded(!isExpanded)}
          disabled={!hasSelection}
          className="ml-2 rounded p-1 hover:bg-muted transition-colors cursor-pointer disabled:opacity-40 disabled:cursor-auto disabled:hover:bg-transparent flex-shrink-0"
          aria-label="Toggle operator details"
        >
          {isExpanded ? (
            <ChevronDown className="h-3 w-3 text-muted-foreground" />
          ) : (
            <ChevronUp className="h-3 w-3 text-muted-foreground" />
          )}
        </button>
      </div>

      {isExpanded &&
        hasSelection &&
        (showDataFlowTab ? (
          <Tabs
            value={activeTab}
            onValueChange={setActiveTab}
            className={cn(
              'border-t',
              fillHeight ? 'min-h-0 flex-1 overflow-hidden' : 'overflow-visible'
            )}
          >
            <TabsList ref={tabsListRef} className="h-7 py-0 px-1 rounded-none">
              <TabsTrigger value="stats" className="text-xs px-2 py-0.5">
                Stats
              </TabsTrigger>
              <TabsTrigger value="data-flow" className="text-xs px-2 py-0.5">
                Data Flow
              </TabsTrigger>
            </TabsList>
            <TabsContent value="stats" className={scrollClass}>
              {statsContent}
            </TabsContent>
            <TabsContent value="data-flow" className={scrollClass}>
              {dataFlowContent}
            </TabsContent>
          </Tabs>
        ) : (
          <div className={cn('border-t', scrollClass)}>{statsContent}</div>
        ))}
    </div>
  );
};
