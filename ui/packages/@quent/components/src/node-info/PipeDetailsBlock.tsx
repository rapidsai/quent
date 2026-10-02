// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect } from 'react';
import { useDisplayedDagNodes, useSetHighlightedNodeIds } from '@quent/hooks';
import { OperatorColorBar } from './OperatorColorBar';
import type { DAGEdge } from '@quent/utils';
import { StatisticFields } from './StatisticFields';

export function PipeDetailsBlock({ pipe }: { pipe: DAGEdge }) {
  const nodes = useDisplayedDagNodes();
  const setHighlight = useSetHighlightedNodeIds();
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
          statistics: pipe.portStats,
        },
        {
          heading: 'Receiving port',
          name: pipe.targetPortName,
          id: pipe.targetPortId,
          operator: pipe.target,
          statistics: pipe.targetPortStats,
        },
      ].map(endpoint => (
        <section
          key={endpoint.heading}
          aria-label={endpoint.heading}
          onMouseEnter={() => enter(endpoint.operator)}
          onMouseLeave={() => leave(endpoint.operator)}
          onFocus={() => enter(endpoint.operator)}
          onBlur={() => leave(endpoint.operator)}
          className="rounded-sm hover:bg-muted/30 focus-within:bg-muted/30"
        >
          <h4 className="mb-1 text-xs font-semibold">
            {endpoint.heading}
            {endpoint.name ? ` · ${endpoint.name}` : ''}
          </h4>
          <dl className="mb-2 grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 text-xs text-muted-foreground">
            <dt>Operator</dt>
            <dd className="flex min-w-0 items-center gap-1.5" title={endpoint.operator}>
              <OperatorColorBar
                operationType={
                  nodes.find(node => node.id === endpoint.operator)?.type ?? 'operator'
                }
                className="w-1 self-stretch"
              />
              <span tabIndex={0} className="break-words text-foreground">
                {nodes.find(node => node.id === endpoint.operator)?.label ?? endpoint.operator}
              </span>
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
