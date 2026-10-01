// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { DAGEdge } from '@quent/utils';
import { StatisticFields } from './StatisticFields';

export function PipeDetailsBlock({ pipe }: { pipe: DAGEdge }) {
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
        <section key={endpoint.heading} aria-label={endpoint.heading}>
          <h4 className="mb-1 text-xs font-semibold">
            {endpoint.heading}
            {endpoint.name ? ` · ${endpoint.name}` : ''}
          </h4>
          <dl className="mb-2 grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 text-xs text-muted-foreground">
            <dt>Operator</dt>
            <dd className="break-all">{endpoint.operator}</dd>
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
