// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Fragment, useEffect, useMemo, type ReactNode } from 'react';
import { ArrowRight, Filter } from 'lucide-react';
import {
  useInspectedPipeNodes,
  useOperatorSelection,
  useOperatorSelectionActions,
  useSelectedOperatorIds,
  useSetHighlightedNodeIds,
} from '@quent/hooks';
import { toggleOperatorSelection, type DAGEdge, type DAGNode, type Operator } from '@quent/utils';
import { operatorSelectionFromNode } from '../dag/dagSelection';
import { OperatorBadge } from '../timeline/OperatorBadge';
import { DataText } from '../ui/data-text';
import { DetailsEntry } from './DetailsEntry';
import { OperatorColorBar } from './OperatorColorBar';
import { StatisticFields } from './StatisticFields';

export interface PipeDetailsBlockProps {
  pipe: DAGEdge;
  /** Controlled open state; the block manages its own state when omitted. */
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  actions?: ReactNode;
}

const operatorSecondaryLabel = (node: DAGNode | undefined, operatorId: string) => {
  const metadata = node?.metadata as
    | {
        operatorWorkerLabels?: Record<string, string | undefined>;
        operatorPlanLabels?: Record<string, string | undefined>;
      }
    | undefined;
  return metadata?.operatorWorkerLabels?.[operatorId] ?? metadata?.operatorPlanLabels?.[operatorId];
};

export function PipeDetailsBlock({ pipe, open, onOpenChange, actions }: PipeDetailsBlockProps) {
  const nodes = useInspectedPipeNodes();
  const setHighlight = useSetHighlightedNodeIds();
  const operatorSelection = useOperatorSelection();
  const updateOperatorSelection = useOperatorSelectionActions();
  const selectedOperatorIds = useSelectedOperatorIds();
  const hierarchyOperators = useMemo(() => {
    const byId = new Map<string, Operator>();
    for (const node of nodes.values()) {
      const metadata = node.metadata as {
        rawNode?: Operator;
        relatedOperators?: readonly Operator[];
      };
      if (metadata?.rawNode) {
        byId.set(metadata.rawNode.id, metadata.rawNode);
      }
      for (const operator of metadata?.relatedOperators ?? []) {
        byId.set(operator.id, operator);
      }
    }
    return [...byId.values()];
  }, [nodes]);
  const selectOperator = (node: DAGNode | undefined) => {
    if (node) {
      const selection = operatorSelectionFromNode(node);
      const filtered = [...selection.operatorIds].every(id => selectedOperatorIds.has(id));
      if (filtered) {
        updateOperatorSelection({
          type: 'replace',
          detailAttention: 'none',
          selections: toggleOperatorSelection(
            hierarchyOperators,
            selectedOperatorIds,
            operatorSelection.selections,
            selection.selectionId
          ),
        });
      } else {
        updateOperatorSelection({
          type: 'add',
          selectionId: selection.selectionId,
          label: selection.label,
          operatorIds: selection.operatorIds,
          selectedData: selection.selectedData,
          detailAttention: 'none',
        });
      }
    }
  };
  useEffect(
    () => () =>
      setHighlight(prev =>
        prev.source === 'dag' &&
        (prev.primaryOperatorId === pipe.source || prev.primaryOperatorId === pipe.target)
          ? { ...prev, ids: null, source: null, primaryOperatorId: null }
          : prev
      ),
    [pipe.source, pipe.target, setHighlight]
  );
  const enter = (id: string) =>
    setHighlight(prev => ({ ...prev, ids: new Set([id]), source: 'dag', primaryOperatorId: id }));
  const leave = (id: string) =>
    setHighlight(prev =>
      prev.source === 'dag' && prev.primaryOperatorId === id
        ? { ...prev, ids: null, source: null, primaryOperatorId: null }
        : prev
    );
  const sourceNode = nodes.get(pipe.source);
  const targetNode = nodes.get(pipe.target);
  const sourceLabel = sourceNode?.label ?? pipe.source;
  const targetLabel = targetNode?.label ?? pipe.target;
  const sourceSecondaryLabel = operatorSecondaryLabel(sourceNode, pipe.source);
  const targetSecondaryLabel = operatorSecondaryLabel(targetNode, pipe.target);
  const pipeSecondaryLabel =
    sourceSecondaryLabel === targetSecondaryLabel
      ? sourceSecondaryLabel
      : [sourceSecondaryLabel, targetSecondaryLabel].filter(Boolean).join(' → ');
  return (
    <DetailsEntry
      open={open}
      onOpenChange={onOpenChange}
      actions={actions}
      testId={`pipe-details-${pipe.id}`}
      triggerLabel={`Toggle pipe from ${sourceLabel} to ${targetLabel} details`}
      bar={<span aria-hidden className="w-[3px] shrink-0 self-stretch rounded-full bg-border" />}
      header={
        <span className="flex min-w-0 flex-1 flex-col gap-1">
          <span className="flex min-w-0 flex-wrap items-center gap-1.5">
            <OperatorBadge label={sourceLabel} operationType={sourceNode?.type} />
            <ArrowRight aria-hidden className="size-4 shrink-0 text-muted-foreground" />
            <OperatorBadge label={targetLabel} operationType={targetNode?.type} />
          </span>
          {pipeSecondaryLabel && (
            <DataText
              className="truncate text-[10px] text-muted-foreground"
              title={pipeSecondaryLabel}
            >
              {pipeSecondaryLabel}
            </DataText>
          )}
        </span>
      }
    >
      <div className="space-y-2">
        {[
          {
            heading: 'Sending port',
            name: pipe.sourcePortName,
            id: pipe.sourcePortId,
            operator: pipe.source,
            node: sourceNode,
            statistics: pipe.portStats,
          },
          {
            heading: 'Receiving port',
            name: pipe.targetPortName,
            id: pipe.targetPortId,
            operator: pipe.target,
            node: targetNode,
            statistics: pipe.targetPortStats,
          },
        ].map((endpoint, index) => {
          const selection = endpoint.node ? operatorSelectionFromNode(endpoint.node) : null;
          const filtered =
            selection !== null &&
            [...selection.operatorIds].every(id => selectedOperatorIds.has(id));
          return (
            <Fragment key={endpoint.heading}>
              {index > 0 && <hr className="border-dashed border-muted-foreground/25" />}
              <section
                aria-label={endpoint.heading}
                onClick={() => selectOperator(endpoint.node)}
                onMouseEnter={() => enter(endpoint.operator)}
                onMouseLeave={() => leave(endpoint.operator)}
                onFocus={() => enter(endpoint.operator)}
                onBlur={() => leave(endpoint.operator)}
                className="cursor-pointer p-2 rounded-sm hover:bg-muted/30 focus-within:bg-muted/30"
              >
                <div className="relative mb-1 flex items-center gap-2 pr-6">
                  <h4 className="text-xs font-semibold">
                    {endpoint.heading}
                    {endpoint.name ? ` · ${endpoint.name}` : ''}
                  </h4>
                  {selection && filtered && (
                    <span
                      role="status"
                      aria-label={`${selection.label}: filtered`}
                      title="Filtered by operator"
                      className="absolute right-0 top-1/2 inline-flex size-6 -translate-y-1/2 items-center justify-center text-foreground"
                    >
                      <Filter aria-hidden className="size-3 opacity-40" />
                    </span>
                  )}
                </div>
                <dl className="mb-2 grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 text-xs text-muted-foreground">
                  <dt>Operator</dt>
                  <dd className="flex min-w-0 items-center gap-1.5" title={endpoint.operator}>
                    <OperatorColorBar
                      operationType={endpoint.node?.type ?? 'operator'}
                      className="w-1 self-stretch"
                    />
                    <button
                      type="button"
                      aria-label={`Filter by ${endpoint.node?.label ?? endpoint.operator} operator`}
                      aria-pressed={filtered}
                      disabled={!endpoint.node}
                      className="cursor-pointer break-words text-left text-foreground rounded-sm focus-visible:outline focus-visible:outline-2 focus-visible:outline-primary"
                    >
                      {endpoint.node?.label ?? endpoint.operator}
                    </button>
                  </dd>
                  <dt>Port ID</dt>
                  <dd className="break-all">{endpoint.id}</dd>
                </dl>
                {endpoint.statistics?.length ? (
                  <StatisticFields statistics={endpoint.statistics} />
                ) : (
                  <p className="text-xs text-muted-foreground">No statistics</p>
                )}
              </section>
            </Fragment>
          );
        })}
      </div>
    </DetailsEntry>
  );
}
