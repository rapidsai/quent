// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useState } from 'react';
import { ChevronDown, ChevronUp } from 'lucide-react';
import {
  useDataFlowEnabled,
  useDataFlowFrame,
  useDataFlowIsPlaying,
  useDataFlowMeta,
  useSelectedNodeData,
} from '@quent/hooks';
import { cn, type QuantitySpec } from '@quent/utils';
import { OperatorColorBar, OperatorDataFlowBlock, OperatorDetailsBlock } from '../node-info';
import { DataText } from '../ui/data-text';
import { thinScrollbarClass } from '../ui/thin-scroll';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../ui/tabs';

export const DAGNodeInfoPanel = ({
  isDark = false,
  quantitySpecs,
}: {
  isDark?: boolean;
  quantitySpecs?: { [key: string]: QuantitySpec | undefined };
}) => {
  const selectedNodeData = useSelectedNodeData();
  const dataFlowEnabled = useDataFlowEnabled();
  const isPlaying = useDataFlowIsPlaying();
  const dataFlowMeta = useDataFlowMeta();
  const dataFlowFrame = useDataFlowFrame();
  const [isExpanded, setIsExpanded] = useState(false);
  const [activeTab, setActiveTab] = useState('stats');
  const [closedOperatorIds, setClosedOperatorIds] = useState<Set<string>>(() => new Set());
  const selectedNodeId = selectedNodeData?.nodeId;
  const hasSelection = selectedNodeData != null;

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

  useEffect(() => {
    setIsExpanded(hasSelection);
    setActiveTab('stats');
    setClosedOperatorIds(new Set());
  }, [hasSelection, selectedNodeId]);

  useEffect(() => {
    if (isPlaying && isExpanded && showDataFlowTab) {
      setActiveTab('data-flow');
    }
  }, [isPlaying, isExpanded, showDataFlowTab]);

  const scrollClass = cn('px-4 pb-2 h-48 overflow-auto', thinScrollbarClass);

  const statsContent = selectedNodeData ? (
    <div className="flex flex-col gap-1 pr-2 pt-1.5">
      <OperatorDetailsBlock
        operator={selectedNodeData}
        quantitySpecs={quantitySpecs}
        isOpen={isOperatorOpen}
        onOpenChange={setOperatorOpen}
      />
    </div>
  ) : null;

  const dataFlowContent =
    selectedNodeData && dataFlowMeta && dataFlowFrame ? (
      <div className="flex flex-col">
        <OperatorDataFlowBlock
          operator={selectedNodeData}
          meta={dataFlowMeta}
          frame={dataFlowFrame}
          isDark={isDark}
          isOpen={isOperatorOpen}
          onOpenChange={setOperatorOpen}
        />
      </div>
    ) : (
      <p className="pt-6 text-xs text-muted-foreground text-center">No tasks at this bin</p>
    );

  return (
    <div className="border-t bg-card flex-shrink-0">
      <div className="flex items-center justify-between px-4 py-1.5 min-w-0">
        <div className="flex items-center gap-2 min-w-0 overflow-hidden">
          <span className="text-xs text-muted-foreground font-medium flex-shrink-0">
            Operator Details
          </span>
          {selectedNodeData && (
            <>
              <span className="text-muted-foreground text-xs flex-shrink-0">·</span>
              <div
                data-testid="operator-details-title"
                className="flex min-w-0 items-center gap-1.5 overflow-hidden"
              >
                <OperatorColorBar
                  operationType={selectedNodeData.operationType}
                  className="h-3 w-1"
                />
                <DataText className="text-xs font-medium truncate" title={selectedNodeData.label}>
                  {selectedNodeData.label}
                </DataText>
                <DataText className="text-xs text-muted-foreground capitalize px-1.5 py-0.5 bg-muted rounded flex-shrink-0">
                  {selectedNodeData.operationType}
                </DataText>
              </div>
            </>
          )}
        </div>
        <button
          onClick={() => setIsExpanded(!isExpanded)}
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
            className="border-t overflow-visible"
          >
            <TabsList className="h-7 py-0 px-1 rounded-none">
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
