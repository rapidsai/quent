// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useState, type ReactNode } from 'react';
import { ChevronUp, ChevronDown } from 'lucide-react';
import {
  useSelectedNodeData,
  useDataFlowEnabled,
  useDataFlowIsPlaying,
  useDataFlowMeta,
  useDataFlowFrame,
  type DataFlowMeta,
  type DataFlowFrame,
  type InspectedNodeData,
  type InspectedOperatorData,
} from '@quent/hooks';
import { DataText } from '../ui/data-text';
import { thinScrollbarClass } from '../ui/thin-scroll';
import { cn, formatStatWithQuantity, getOperationTypeColor, type QuantitySpec } from '@quent/utils';
import { DataFlowMatrix } from './DataFlowMatrix';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '../ui/tabs';
import { Collapsible, CollapsibleTrigger, CollapsibleContent } from '../ui/collapsible';

const OperatorColorBar = ({
  operationType,
  className,
}: {
  operationType: string;
  className?: string;
}) => (
  <span
    aria-hidden
    data-testid="operator-color-bar"
    data-operation-type={operationType}
    className={cn('shrink-0 rounded-full', className)}
    style={{ backgroundColor: getOperationTypeColor(operationType) }}
  />
);

const OperatorStatFields = ({
  operator,
  quantitySpecs,
}: {
  operator: InspectedOperatorData;
  quantitySpecs?: { [key: string]: QuantitySpec | undefined };
}) => (
  <>
    <div className="text-xs flex items-center justify-between">
      <DataText className="capitalize">ID:</DataText>
      <DataText className="text-muted-foreground ml-1 truncate">{operator.nodeId}</DataText>
    </div>
    {operator.statistics.map(({ key, value, quantity }) => (
      <div key={key} className="text-xs">
        {Array.isArray(value) ? (
          <div className="flex items-center justify-between gap-0.5">
            <DataText className="capitalize">{key.replace(/_/g, ' ')}:</DataText>
            <div className="ml-2 flex flex-col gap-0.5">
              {value.map((item, i) => (
                <DataText key={i} className="text-muted-foreground whitespace-pre-line">
                  {String(item)}
                </DataText>
              ))}
            </div>
          </div>
        ) : (
          <div className="flex items-center justify-between">
            <DataText className="capitalize">{key.replace(/_/g, ' ')}:</DataText>
            <DataText className="text-muted-foreground ml-1">
              {typeof value === 'number'
                ? formatStatWithQuantity(
                    value,
                    key,
                    quantity && quantitySpecs ? quantitySpecs[quantity] : undefined
                  )
                : String(value)}
            </DataText>
          </div>
        )}
      </div>
    ))}
  </>
);

const OperatorAccordion = ({
  operator,
  open,
  onOpenChange,
  children,
}: {
  operator: InspectedOperatorData;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  children: ReactNode;
}) => (
  <Collapsible
    open={open}
    onOpenChange={onOpenChange}
    className="flex min-w-0 gap-2"
    data-testid={`operator-accordion-${operator.nodeId}`}
  >
    <OperatorColorBar operationType={operator.operationType} className="w-1 self-stretch" />
    <div className="min-w-0 flex-1">
      <CollapsibleTrigger
        className="group flex w-full min-w-0 cursor-pointer items-center gap-2 rounded-sm px-1.5 py-1 my-1 text-left hover:bg-muted/50"
        aria-label={`Toggle ${operator.label} details`}
      >
        <DataText className="min-w-0 truncate text-xs font-medium" title={operator.label}>
          {operator.label}
        </DataText>
        <DataText className="shrink-0 rounded bg-muted px-1.5 py-0.5 text-xs capitalize text-muted-foreground">
          {operator.operationType}
        </DataText>
        <ChevronDown className="ml-auto h-3 w-3 shrink-0 text-muted-foreground transition-transform group-data-[state=closed]:-rotate-90" />
      </CollapsibleTrigger>
      <CollapsibleContent>{children}</CollapsibleContent>
    </div>
  </Collapsible>
);

const OperatorDetailsBlock = ({
  operator,
  quantitySpecs,
  isOpen,
  onOpenChange,
}: {
  operator: InspectedNodeData;
  quantitySpecs?: { [key: string]: QuantitySpec | undefined };
  isOpen: (id: string) => boolean;
  onOpenChange: (id: string, open: boolean) => void;
}) => (
  <OperatorAccordion
    operator={operator}
    open={isOpen(operator.nodeId)}
    onOpenChange={open => onOpenChange(operator.nodeId, open)}
  >
    <OperatorStatFields operator={operator} quantitySpecs={quantitySpecs} />
    {operator.relatedOperators?.map(related => (
      <div key={related.nodeId} className="mt-1.5 border-t pt-1.5">
        <OperatorAccordion
          operator={related}
          open={isOpen(related.nodeId)}
          onOpenChange={open => onOpenChange(related.nodeId, open)}
        >
          <OperatorStatFields operator={related} quantitySpecs={quantitySpecs} />
        </OperatorAccordion>
      </div>
    ))}
  </OperatorAccordion>
);

const OperatorDataFlowBlock = ({
  operator,
  meta,
  frame,
  isDark,
  isOpen,
  onOpenChange,
}: {
  operator: InspectedNodeData;
  meta: DataFlowMeta;
  frame: DataFlowFrame;
  isDark: boolean;
  isOpen: (id: string) => boolean;
  onOpenChange: (id: string, open: boolean) => void;
}) => (
  <OperatorAccordion
    operator={operator}
    open={isOpen(operator.nodeId)}
    onOpenChange={open => onOpenChange(operator.nodeId, open)}
  >
    <DataFlowMatrix
      meta={meta}
      frame={frame}
      operatorFrame={frame.perOperator.get(operator.nodeId)}
      isDark={isDark}
    />
    {operator.relatedOperators?.map(related => (
      <div key={related.nodeId} className="mt-1.5 border-t pt-1.5">
        <OperatorAccordion
          operator={related}
          open={isOpen(related.nodeId)}
          onOpenChange={open => onOpenChange(related.nodeId, open)}
        >
          <DataFlowMatrix
            meta={meta}
            frame={frame}
            operatorFrame={frame.perOperator.get(related.nodeId)}
            isDark={isDark}
          />
        </OperatorAccordion>
      </div>
    ))}
  </OperatorAccordion>
);

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
