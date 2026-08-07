// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { formatAttributeValue, formatDuration } from '@quent/utils';
import type { DynamicAttribute, FiniteStateMachine } from '@quent/utils';

interface EntityDetailPanelProps {
  fsm: FiniteStateMachine | null;
  resourceLabel: (id: string) => string;
}

export function EntityDetailPanel({ fsm, resourceLabel }: EntityDetailPanelProps) {
  if (!fsm) {
    return (
      <div className="flex h-full items-center justify-center p-4 text-center text-sm text-muted-foreground">
        Select an entity to view its states.
      </div>
    );
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="shrink-0 border-b bg-card p-3">
        <div className="text-sm font-medium">{fsm.instance_name}</div>
        <div className="text-xs text-muted-foreground">{fsm.type_name}</div>
        <div className="mt-1 break-all font-mono text-xs text-muted-foreground">{fsm.id}</div>
      </div>
      <ol className="min-h-0 flex-1 space-y-2 overflow-auto p-3">
        {fsm.transitions.map((transition, index) => {
          const nextTransition = fsm.transitions[index + 1];
          return (
            <li key={index} className="rounded border bg-card p-2">
              <div className="flex items-baseline justify-between gap-2">
                <span className="text-sm font-medium">
                  <span className="text-muted-foreground">{index + 1}.</span> {transition.name}
                </span>
                <span className="tabular-nums text-xs text-muted-foreground">
                  {transition.timestamp.toFixed(3)}s
                  {nextTransition && (
                    <>
                      {' '}
                      · for{' '}
                      {formatDuration((nextTransition.timestamp - transition.timestamp) * 1000)}
                    </>
                  )}
                </span>
              </div>
              {transition.usages.length > 0 && (
                <ul className="mt-1 space-y-0.5 text-xs text-muted-foreground">
                  {transition.usages.map((usage, usageIndex) => (
                    <li key={usageIndex} className="flex flex-wrap items-baseline gap-x-2">
                      <span className="font-mono">{resourceLabel(usage.resource)}</span>
                      {usage.capacities.map(([name, capacity], capacityIndex) => (
                        <span key={capacityIndex} className="tabular-nums">
                          {name}
                          {capacity != null ? `=${capacity}` : ''}
                        </span>
                      ))}
                    </li>
                  ))}
                </ul>
              )}
              {transition.attributes.length > 0 && (
                <AttributeRows attributes={transition.attributes} />
              )}
              {transition.derived_attributes.length > 0 && (
                <AttributeRows attributes={transition.derived_attributes} derived />
              )}
            </li>
          );
        })}
      </ol>
    </div>
  );
}

function AttributeRows({ attributes, derived }: { attributes: DynamicAttribute[]; derived?: boolean }) {
  return (
    <ul className={`mt-1 space-y-0.5 text-xs ${derived ? 'italic text-muted-foreground' : ''}`}>
      {attributes.map((attribute, index) => (
        <li key={index} className="flex justify-between gap-3">
          <span className={derived ? '' : 'text-muted-foreground'}>{attribute.key}</span>
          <span className="tabular-nums">
            {formatAttributeValue(attribute.key, attribute.value)}
          </span>
        </li>
      ))}
    </ul>
  );
}
