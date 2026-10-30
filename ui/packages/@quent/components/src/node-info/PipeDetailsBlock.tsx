// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Fragment, useEffect, type ReactNode } from 'react';
import { Filter } from 'lucide-react';
import {
  useInspectedPipeNodes,
  useOperatorSelection,
  useOperatorSelectionActions,
  useSelectedOperatorIds,
  useSetHighlightedNodeIds,
} from '@quent/hooks';
import type { DAGEdge, DAGNode } from '@quent/utils';
import { operatorSelectionFromNode } from '../dag/dagSelection';
import { OperatorBadge } from '../timeline/OperatorBadge';
import { DetailsEntry } from './DetailsEntry';
import { OperatorColorBar } from './OperatorColorBar';
import { StatisticFields } from './StatisticFields';

export interface PipeDetailsBlockProps {
  pipe: DAGEdge;
  /** Controlled open state; the block manages its own state when omitted. */
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  actions?: ReactNode;
  onOperatorFilterAdded?: () => void;
}

export function PipeDetailsBlock({
  pipe,
  open,
  onOpenChange,
  actions,
  onOperatorFilterAdded,
}: PipeDetailsBlockProps) {
  const nodes = useInspectedPipeNodes();
  const setHighlight = useSetHighlightedNodeIds();
  const operatorSelection = useOperatorSelection();
  const updateOperatorSelection = useOperatorSelectionActions();
  const selectedOperatorIds = useSelectedOperatorIds();
  const selectOperator = (node: DAGNode | undefined) => {
    if (node) {
      const selection = operatorSelectionFromNode(node);
      const matchingFilterIds = [...operatorSelection.selections]
        .filter(
          ([selectionId, current]) =>
            selectionId === selection.selectionId ||
            (current.operatorIds.size === selection.operatorIds.size &&
              [...current.operatorIds].every(id => selection.operatorIds.has(id)))
        )
        .map(([selectionId]) => selectionId);
      const filtered = [...selection.operatorIds].every(id => selectedOperatorIds.has(id));
      if (filtered && matchingFilterIds.length > 0) {
        for (const selectionId of matchingFilterIds) {
          updateOperatorSelection({ type: 'remove', selectionId });
        }
      } else {
        onOperatorFilterAdded?.();
        updateOperatorSelection({
          type: 'add',
          selectionId: selection.selectionId,
          label: selection.label,
          operatorIds: selection.operatorIds,
          selectedData: selection.selectedData,
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
  return (
    <DetailsEntry
      open={open}
      onOpenChange={onOpenChange}
      actions={actions}
      testId={`pipe-details-${pipe.id}`}
      triggerLabel={`Toggle pipe from ${sourceLabel} to ${targetLabel} details`}
      bar={<span aria-hidden className="w-[3px] shrink-0 self-stretch rounded-full bg-border" />}
      header={
        <span className="flex min-w-0 flex-1 flex-wrap items-center gap-1.5">
          <OperatorBadge label={sourceLabel} operationType={sourceNode?.type} />
          <span aria-hidden className="text-muted-foreground">
            →
          </span>
          <OperatorBadge label={targetLabel} operationType={targetNode?.type} />
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
                      title="Filtered"
                      className="pointer-events-none absolute right-0 top-1/2 inline-flex size-6 -translate-y-1/2 items-center justify-center text-primary"
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
