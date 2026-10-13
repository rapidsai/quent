// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect } from 'react';
import {
  useDisplayedDagNodes,
  useSetHighlightedNodeIds,
  useOperatorSelectionActions,
} from '@quent/hooks';
import type { DAGEdge, DAGNode } from '@quent/utils';
import { operatorSelectionFromNode } from '../dag/dagSelection';
import { OperatorColorBar } from './OperatorColorBar';
import { StatisticFields } from './StatisticFields';

export function PipeDetailsBlock({ pipe }: { pipe: DAGEdge }) {
  const nodes = useDisplayedDagNodes();
  const setHighlight = useSetHighlightedNodeIds();
  const updateSelection = useOperatorSelectionActions();
  const selectOperator = (node: DAGNode | undefined) => {
    if (node) {
      updateSelection({ type: 'add', ...operatorSelectionFromNode(node) });
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
  return (
    <div className="space-y-4">
      {[
        {
          heading: 'Sending port',
          name: pipe.sourcePortName,
          id: pipe.sourcePortId,
          operator: pipe.source,
          node: nodes.find(node => node.id === pipe.source),
          statistics: pipe.portStats,
        },
        {
          heading: 'Receiving port',
          name: pipe.targetPortName,
          id: pipe.targetPortId,
          operator: pipe.target,
          node: nodes.find(node => node.id === pipe.target),
          statistics: pipe.targetPortStats,
        },
      ].map(endpoint => (
        <section
          key={endpoint.heading}
          aria-label={endpoint.heading}
          onClick={() => selectOperator(endpoint.node)}
          onMouseEnter={() => enter(endpoint.operator)}
          onMouseLeave={() => leave(endpoint.operator)}
          onFocus={() => enter(endpoint.operator)}
          onBlur={() => leave(endpoint.operator)}
          className="cursor-pointer rounded-sm hover:bg-muted/30 focus-within:bg-muted/30"
        >
          <h4 className="mb-1 text-xs font-semibold">
            {endpoint.heading}
            {endpoint.name ? ` · ${endpoint.name}` : ''}
          </h4>
          <dl className="mb-2 grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 text-xs text-muted-foreground">
            <dt>Operator</dt>
            <dd className="flex min-w-0 items-center gap-1.5" title={endpoint.operator}>
              <OperatorColorBar
                operationType={endpoint.node?.type ?? 'operator'}
                className="w-1 self-stretch"
              />
              <button
                type="button"
                aria-label={`Show ${endpoint.node?.label ?? endpoint.operator} operator details`}
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
      ))}
    </div>
  );
}
