// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useRef, useState } from 'react';
import { Check, Copy } from 'lucide-react';
import { DataText, FsmCapacityChart, SegmentedBar, thinScrollbarClass } from '@quent/components';
import {
  cn,
  formatBytes,
  formatDuration,
  formatDurationForWindow,
  getColorForKey,
  isBytesStat,
  unwrapTaggedValue,
} from '@quent/utils';
import type { EntityRef, FiniteStateMachine, QueryBundle } from '@quent/utils';
import { useTheme, THEME_DARK } from '@/contexts/ThemeContext';
import { ResourceUsageList } from './ResourceUsageList';
import { TransitionAttributes } from './TransitionAttributes';

interface EntityDetailPanelProps {
  fsm: FiniteStateMachine | null;
  resourceLabel: (id: string) => string;
  operatorLabel: (id: string) => string;
  stateColorFn?: (name: string) => string;
  queryBundle: QueryBundle<EntityRef>;
}

export function EntityDetailPanel({
  fsm,
  resourceLabel,
  operatorLabel,
  stateColorFn,
  queryBundle,
}: EntityDetailPanelProps) {
  const { theme } = useTheme();
  const paletteTheme = theme === THEME_DARK ? ('dark' as const) : ('light' as const);
  const [copied, setCopied] = useState(false);
  const copiedTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (copiedTimeoutRef.current != null) {
        clearTimeout(copiedTimeoutRef.current);
      }
    };
  }, []);

  if (!fsm) {
    return (
      <div className="flex h-full items-center justify-center p-4 text-center text-sm text-muted-foreground">
        Select an entity to view its states.
      </div>
    );
  }

  const fsmId = fsm.id;
  const firstTs = fsm.transitions[0]?.timestamp ?? 0;
  const lastTs = fsm.transitions[fsm.transitions.length - 1]?.timestamp ?? firstTs;
  const totalSpanMs = (lastTs - firstTs) * 1000;

  // Precompute per-transition durations (null for the final state)
  const durations = fsm.transitions.map((t, i) => {
    const next = fsm.transitions[i + 1];
    return next ? (next.timestamp - t.timestamp) * 1000 : null;
  });

  // Aggregate total time per state name (insertion order = first appearance)
  const stateTimeMs = new Map<string, number>();
  fsm.transitions.forEach((t, i) => {
    const d = durations[i];
    if (d != null) {
      stateTimeMs.set(t.name, (stateTimeMs.get(t.name) ?? 0) + d);
    }
  });

  // Find the state that consumed the most time
  let dominantState: { name: string; pct: number; color: string } | null = null;
  if (totalSpanMs > 0 && stateTimeMs.size > 0) {
    let maxMs = 0;
    let maxName = '';
    stateTimeMs.forEach((ms, name) => {
      if (ms > maxMs) {
        maxMs = ms;
        maxName = name;
      }
    });
    dominantState = {
      name: maxName,
      pct: (maxMs / totalSpanMs) * 100,
      color: stateColorFn ? stateColorFn(maxName) : getColorForKey(maxName, paletteTheme),
    };
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
    void navigator.clipboard.writeText(fsmId);
    setCopied(true);
    if (copiedTimeoutRef.current != null) {
      clearTimeout(copiedTimeoutRef.current);
    }
    copiedTimeoutRef.current = setTimeout(() => setCopied(false), 1500);
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      {/* Compact header: name + type badge on one line, UUID + copy on second */}
      <div className="shrink-0 border-b bg-card px-3 py-2">
        <div className="flex items-center gap-2">
          <DataText className="text-sm font-medium">{fsm.instance_name}</DataText>
          <DataText className="rounded bg-muted px-1.5 py-0.5 text-xs text-muted-foreground">
            {fsm.type_name}
          </DataText>
        </div>
        <div className="mt-1 flex items-center gap-1">
          <DataText className="min-w-0 flex-1 truncate text-xs text-muted-foreground">
            {fsm.id}
          </DataText>
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
          <DataText className="tabular-nums font-medium">{formatDuration(totalSpanMs)}</DataText>
        </div>
        {dominantState && (
          <div className="mt-0.5 flex items-center justify-between gap-2">
            <span className="text-muted-foreground">Dominant state</span>
            <DataText className="font-medium" style={{ color: dominantState.color }}>
              {dominantState.name} · {dominantState.pct.toFixed(1)}%
            </DataText>
          </div>
        )}
        {dataVolume && (
          <div className="mt-0.5 flex items-center justify-between gap-2">
            <span className="text-muted-foreground">Data volume</span>
            <DataText className="tabular-nums font-medium">{dataVolume}</DataText>
          </div>
        )}
        {totalSpanMs > 0 && stateTimeMs.size > 0 && (
          <SegmentedBar
            className="mt-2"
            trackClassName="rounded-full bg-transparent"
            height={8}
            showLabels={false}
            showTooltips
            segments={[...stateTimeMs.entries()].map(([name, ms]) => {
              const color = stateColorFn ? stateColorFn(name) : getColorForKey(name, paletteTheme);
              const pct = (ms / totalSpanMs) * 100;
              return {
                id: name,
                value: pct,
                color,
                ariaLabel: `${name}: ${pct.toFixed(1)}%`,
                tooltip: (
                  <div className="rounded bg-popover px-2 py-1.5 text-[11px] leading-tight text-foreground shadow-md">
                    <DataText className="font-medium">{name}</DataText>
                    <DataText className="ml-2 text-muted-foreground">{pct.toFixed(1)}%</DataText>
                  </div>
                ),
              };
            })}
          />
        )}
      </div>

      <FsmCapacityChart
        transitions={fsm.transitions}
        isDark={theme === THEME_DARK}
        resourceLabel={resourceLabel}
        quantitySpecs={queryBundle.quantity_specs}
        defaultCapacityPredicate={isBytesStat}
        getCapacityDecl={(resourceId, capacityName) => {
          const typeName = queryBundle.entities.resources[resourceId]?.type_name;
          const resourceType = typeName ? queryBundle.entities.resource_types[typeName] : undefined;
          return resourceType?.capacities.find(c => c.name === capacityName);
        }}
      />

      <ol className={cn('min-h-0 flex-1 space-y-2 overflow-auto p-3', thinScrollbarClass)}>
        {fsm.transitions.map((transition, index) => {
          const durationMs = durations[index] ?? null;
          const isBottleneck =
            durationMs != null && totalSpanMs > 0 && durationMs / totalSpanMs > 0.5;
          const stateColor = stateColorFn
            ? stateColorFn(transition.name)
            : getColorForKey(transition.name, paletteTheme);
          const pct =
            durationMs != null && totalSpanMs > 0
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
                <DataText className="text-sm font-medium">
                  <span className="text-muted-foreground">{index + 1}.</span> {transition.name}
                </DataText>
                <div className="flex shrink-0 flex-col items-end">
                  {durationMs != null && (
                    <DataText
                      className={cn(
                        'tabular-nums text-sm font-medium',
                        isBottleneck && 'text-orange-500 dark:text-orange-400'
                      )}
                    >
                      {formatDuration(durationMs)}
                    </DataText>
                  )}
                  <DataText className="tabular-nums text-xs text-muted-foreground">
                    @{formatDurationForWindow(transition.timestamp * 1000, totalSpanMs, 15)}
                  </DataText>
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

              <ResourceUsageList
                usages={transition.usages}
                resourceLabel={resourceLabel}
                queryBundle={queryBundle}
              />
              <TransitionAttributes
                attributes={transition.attributes}
                derivedAttributes={transition.derived_attributes}
                operatorLabel={operatorLabel}
              />
            </li>
          );
        })}
      </ol>
    </div>
  );
}
