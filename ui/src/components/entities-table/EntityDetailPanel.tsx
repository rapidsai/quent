// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useState } from 'react';
import { Check, Copy } from 'lucide-react';
import { thinScrollbarClass, FsmCapacityChart } from '@quent/components';
import {
  formatAttributeValue,
  formatDuration,
  formatBytes,
  getColorForKey,
  unwrapTaggedValue,
} from '@quent/utils';
import type { DynamicAttribute, FiniteStateMachine } from '@quent/utils';
import { useTheme, THEME_DARK } from '@/contexts/ThemeContext';

interface EntityDetailPanelProps {
  fsm: FiniteStateMachine | null;
  resourceLabel: (id: string) => string;
  operatorLabel: (id: string) => string;
}

function isBytesStat(name: string): boolean {
  return (
    name.includes('_bytes') ||
    name.endsWith('_byte') ||
    name.startsWith('bytes_') ||
    name === 'bytes'
  );
}

export function EntityDetailPanel({ fsm, resourceLabel, operatorLabel }: EntityDetailPanelProps) {
  const { theme } = useTheme();
  const paletteTheme = theme === THEME_DARK ? ('dark' as const) : ('light' as const);
  const [copied, setCopied] = useState(false);

  if (!fsm) {
    return (
      <div className="flex h-full items-center justify-center p-4 text-center text-sm text-muted-foreground">
        Select an entity to view its states.
      </div>
    );
  }

  const firstTs = fsm.transitions[0]?.timestamp ?? 0;
  const lastTs = fsm.transitions[fsm.transitions.length - 1]?.timestamp ?? firstTs;
  const totalSpanMs = (lastTs - firstTs) * 1000;

  // Precompute per-transition durations (null for the final state)
  const durations = fsm.transitions.map((t, i) => {
    const next = fsm.transitions[i + 1];
    return next ? (next.timestamp - t.timestamp) * 1000 : null;
  });

  // Find the state that consumed the most time
  let dominantState: { name: string; pct: number; color: string } | null = null;
  if (totalSpanMs > 0) {
    let maxMs = 0;
    let maxIdx = -1;
    durations.forEach((d, i) => {
      if (d != null && d > maxMs) {
        maxMs = d;
        maxIdx = i;
      }
    });
    if (maxIdx >= 0) {
      const name = fsm.transitions[maxIdx]!.name;
      dominantState = {
        name,
        pct: (maxMs / totalSpanMs) * 100,
        color: getColorForKey(name, paletteTheme),
      };
    }
  }

  // Find data volume from derived attributes (last bytes-stat with a numeric value)
  let dataVolume: string | null = null;
  for (let i = fsm.transitions.length - 1; i >= 0; i--) {
    for (const attr of fsm.transitions[i]!.derived_attributes) {
      if (isBytesStat(attr.key) && attr.value != null) {
        const raw = unwrapTaggedValue(attr.value);
        if (typeof raw === 'number' || typeof raw === 'bigint') {
          dataVolume = formatBytes(raw);
          break;
        }
      }
    }
    if (dataVolume) break;
  }

  function copyId() {
    void navigator.clipboard.writeText(fsm!.id);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      {/* Compact header: name + type badge on one line, UUID + copy on second */}
      <div className="shrink-0 border-b bg-card px-3 py-2">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium">{fsm.instance_name}</span>
          <span className="rounded bg-muted px-1.5 py-0.5 text-xs text-muted-foreground">
            {fsm.type_name}
          </span>
        </div>
        <div className="mt-1 flex items-center gap-1">
          <span className="min-w-0 flex-1 truncate font-mono text-xs text-muted-foreground">
            {fsm.id}
          </span>
          <button
            onClick={copyId}
            aria-label="Copy ID"
            className="shrink-0 rounded p-0.5 text-muted-foreground transition-colors hover:text-foreground"
          >
            {copied ? <Check className="size-3" /> : <Copy className="size-3" />}
          </button>
        </div>
      </div>

      {/* Summary strip */}
      <div className="shrink-0 border-b bg-muted/30 px-3 py-2 text-xs">
        <div className="flex items-center justify-between gap-2">
          <span className="text-muted-foreground">Total span</span>
          <span className="tabular-nums font-medium">{formatDuration(totalSpanMs)}</span>
        </div>
        {dominantState && (
          <div className="mt-0.5 flex items-center justify-between gap-2">
            <span className="text-muted-foreground">Dominant state</span>
            <span className="font-medium" style={{ color: dominantState.color }}>
              {dominantState.name} · {dominantState.pct.toFixed(1)}%
            </span>
          </div>
        )}
        {dataVolume && (
          <div className="mt-0.5 flex items-center justify-between gap-2">
            <span className="text-muted-foreground">Data volume</span>
            <span className="tabular-nums font-medium">{dataVolume}</span>
          </div>
        )}
      </div>

      <FsmCapacityChart
        transitions={fsm.transitions}
        isDark={theme === THEME_DARK}
        resourceLabel={resourceLabel}
      />

      <ol className={`min-h-0 flex-1 space-y-2 overflow-auto p-3 ${thinScrollbarClass}`}>
        {fsm.transitions.map((transition, index) => {
          const durationMs = durations[index] ?? null;
          const isBottleneck =
            durationMs != null && totalSpanMs > 0 && durationMs / totalSpanMs > 0.5;
          const stateColor = getColorForKey(transition.name, paletteTheme);
          const pct = durationMs != null && totalSpanMs > 0
            ? Math.min(100, (durationMs / totalSpanMs) * 100)
            : null;

          return (
            <li
              key={`${index}-${transition.name}`}
              className="rounded border bg-card p-2"
              style={{ borderLeftColor: stateColor, borderLeftWidth: 3 }}
            >
              {/* State name + duration (prominent) + absolute timestamp (secondary) */}
              <div className="flex items-start justify-between gap-2">
                <span className="text-sm font-medium">
                  <span className="text-muted-foreground">{index + 1}.</span> {transition.name}
                </span>
                <div className="flex shrink-0 flex-col items-end">
                  {durationMs != null && (
                    <span
                      className={`tabular-nums text-sm font-medium ${
                        isBottleneck ? 'text-orange-500 dark:text-orange-400' : ''
                      }`}
                    >
                      {formatDuration(durationMs)}
                    </span>
                  )}
                  <span className="tabular-nums text-xs text-muted-foreground">
                    @{transition.timestamp.toFixed(3)}s
                  </span>
                </div>
              </div>

              {/* Proportional duration bar */}
              {pct != null && (
                <div className="mt-1.5 h-1 w-full overflow-hidden rounded-full bg-muted">
                  <div
                    className="h-full rounded-full"
                    style={{ width: `${pct}%`, backgroundColor: stateColor }}
                  />
                </div>
              )}

              {transition.usages.length > 0 && (
                <ul className="mt-1 space-y-0.5 text-xs text-muted-foreground">
                  {transition.usages.map((usage, usageIndex) => (
                    <li key={usageIndex} className="flex flex-wrap items-baseline gap-x-2">
                      <span className="font-mono">{resourceLabel(usage.resource)}</span>
                      {usage.capacities.map(([name, capacity], capacityIndex) => (
                        <span key={capacityIndex} className="tabular-nums">
                          {name}
                          {capacity != null
                            ? `=${isBytesStat(name) ? formatBytes(capacity) : String(capacity)}`
                            : ''}
                        </span>
                      ))}
                    </li>
                  ))}
                </ul>
              )}
              {transition.attributes.length > 0 && (
                <AttributeRows
                  attributes={transition.attributes}
                  operatorLabel={operatorLabel}
                />
              )}
              {transition.derived_attributes.length > 0 && (
                <AttributeRows
                  attributes={transition.derived_attributes}
                  derived
                  operatorLabel={operatorLabel}
                />
              )}
            </li>
          );
        })}
      </ol>
    </div>
  );
}

function AttributeRows({
  attributes,
  derived,
  operatorLabel,
}: {
  attributes: DynamicAttribute[];
  derived?: boolean;
  operatorLabel: (id: string) => string;
}) {
  return (
    <ul className={`mt-1 space-y-0.5 text-xs ${derived ? 'italic text-muted-foreground' : ''}`}>
      {attributes.map((attribute, index) => {
        const { label, value } = resolveAttributeDisplay(attribute, operatorLabel);
        return (
          <li key={index} className="flex justify-between gap-3">
            <span className={derived ? '' : 'text-muted-foreground'}>{label}</span>
            <span className="tabular-nums text-right">{value}</span>
          </li>
        );
      })}
    </ul>
  );
}

function resolveAttributeDisplay(
  attribute: DynamicAttribute,
  operatorLabel: (id: string) => string
): { label: string; value: string } {
  if (attribute.key === 'operator_id') {
    const raw = unwrapTaggedValue(attribute.value);
    if (typeof raw === 'string') {
      return { label: 'operator', value: operatorLabel(raw) };
    }
  }
  return { label: attribute.key, value: formatAttributeValue(attribute.key, attribute.value) };
}
