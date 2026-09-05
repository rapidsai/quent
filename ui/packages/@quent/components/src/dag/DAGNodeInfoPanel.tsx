// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useState } from 'react';
import { ChevronDown, ChevronUp } from 'lucide-react';
import {
  useDataFlowEnabled,
  useDataFlowFrame,
  useDataFlowIsPlaying,
  useDataFlowMeta,
  useSelectedNodesData,
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
  const selectedNodes = useSelectedNodesData();
  const dataFlowEnabled = useDataFlowEnabled();
  const isPlaying = useDataFlowIsPlaying();
  const dataFlowMeta = useDataFlowMeta();
  const dataFlowFrame = useDataFlowFrame();
  const [isExpanded, setIsExpanded] = useState(false);
  const [activeTab, setActiveTab] = useState('stats');
  const [closedOperatorIds, setClosedOperatorIds] = useState<Set<string>>(() => new Set());
  const hasSelection = selectedNodes.length > 0;
  const showHeaders = selectedNodes.length > 1;
  const selectedNode = selectedNodes[0];
  const selectedNodeIdsKey = selectedNodes.map(node => node.nodeId).join('\0');

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
    if (!hasSelection) {
      setActiveTab('stats');
    }
  }, [hasSelection]);

  useEffect(() => {
    setClosedOperatorIds(new Set());
  }, [selectedNodeIdsKey]);

  useEffect(() => {
    if (isPlaying && isExpanded && showDataFlowTab) {
      setActiveTab('data-flow');
    }
  }, [isPlaying, isExpanded, showDataFlowTab]);

  const scrollClass = cn('px-4 pb-2 h-48 overflow-auto', thinScrollbarClass);

  const statsContent = hasSelection ? (
    <div className="flex flex-col gap-1 pr-2 pt-1.5">
      {selectedNodes.map((operator, index) => (
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
      <div className="flex flex-col">
        {selectedNodes.map((operator, index) => (
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
      <p className="pt-6 text-xs text-muted-foreground text-center">No tasks at this bin</p>
    );

  return (
    <div className="border-t bg-card flex-shrink-0">
      <div className="flex items-center justify-between px-4 py-1.5 min-w-0">
        <div className="flex items-center gap-2 min-w-0 overflow-hidden">
          <span className="text-xs text-muted-foreground font-medium flex-shrink-0">
            Operator Details
          </span>
          {selectedNode && (
            <>
              <span className="text-muted-foreground text-xs flex-shrink-0">·</span>
              <div
                data-testid="operator-details-title"
                className="flex min-w-0 items-center gap-1.5 overflow-hidden"
              >
                {selectedNodes.map((operator, index) => (
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
