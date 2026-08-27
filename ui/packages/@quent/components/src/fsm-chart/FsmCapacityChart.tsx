// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useMemo, useState } from 'react';
import EChartsReactCore from 'echarts-for-react/lib/core';
import type { CapacityDecl, FsmTransition, QuantitySpec } from '@quent/utils';
import { bigintToChartNumber, formatBytes, formatQuantity } from '@quent/utils';
import { echarts } from '../lib/echarts';
import { useChartResize } from '../lib/useChartResize';
import type { PointerPosition } from '../ui/pointer-tooltip-portal';
import { PositionedTooltip } from '../ui/positioned-tooltip';
import { SelectField } from '../ui/select-field';
import { useTimelineEchartsTheme } from '../timeline/timelineEchartsTheme';
import { FsmCapacityTooltip } from './FsmCapacityTooltip';

const CHART_HEIGHT = 90;
const GRID = { left: 52, right: 8, top: 8, bottom: 36 };

interface CapacityEntry {
  name: string;
  statLabel: string;
  data: Array<number | null>;
  rawData: Array<bigint | null>;
  formatter: (v: number | bigint) => string;
}

interface ResourceSeries {
  resourceId: string;
  label: string;
  capacities: CapacityEntry[];
}

interface AxisPointerEvent {
  axesInfo?: Array<{ axisDim?: string; value?: number }>;
}

export interface FsmCapacityChartProps {
  transitions: FsmTransition[];
  isDark: boolean;
  resourceLabel: (id: string) => string;
  quantitySpecs: { [key in string]: QuantitySpec };
  getCapacityDecl: (resourceId: string, capacityName: string) => CapacityDecl | undefined;
  defaultCapacityPredicate?: (name: string) => boolean;
}

const SELECT_TRIGGER_CLASS =
  'h-auto w-auto max-w-[140px] gap-1 truncate rounded border border-border bg-background px-1 py-0.5 text-[10px] text-foreground focus:outline-none focus:ring-1 focus:ring-ring [&>svg]:h-3 [&>svg]:w-3';

export function FsmCapacityChart({
  transitions,
  isDark,
  resourceLabel,
  quantitySpecs,
  getCapacityDecl,
  defaultCapacityPredicate,
}: FsmCapacityChartProps) {
  const { themeName } = useTimelineEchartsTheme(isDark);
  const { handleChartReady } = useChartResize();
  const [pointer, setPointer] = useState<PointerPosition | null>(null);
  const [dataIndex, setDataIndex] = useState<number | null>(null);
  const [selectedResourceId, setSelectedResourceId] = useState<string | null>(null);
  const [selectedCapacityName, setSelectedCapacityName] = useState<string | null>(null);

  const { resources, stateLabels } = useMemo(() => {
    const n = transitions.length;
    const stateLabels = transitions.map((t, i) => `${i + 1}. ${t.name}`);

    // Accumulate data keyed by resourceId → capacityName
    const resourceMap = new Map<
      string,
      {
        label: string;
        caps: Map<string, { data: Array<number | null>; rawData: Array<bigint | null> }>;
      }
    >();

    transitions.forEach((t, i) => {
      t.usages.forEach(usage => {
        if (!resourceMap.has(usage.resource)) {
          resourceMap.set(usage.resource, {
            label: resourceLabel(usage.resource),
            caps: new Map(),
          });
        }
        const entry = resourceMap.get(usage.resource)!;
        usage.capacities.forEach(([name, cap]) => {
          if (cap == null) {
            return;
          }
          if (!entry.caps.has(name)) {
            entry.caps.set(name, {
              data: Array<number | null>(n).fill(null),
              rawData: Array<bigint | null>(n).fill(null),
            });
          }
          const capEntry = entry.caps.get(name)!;
          capEntry.data[i] = bigintToChartNumber(cap);
          capEntry.rawData[i] = cap;
        });
      });
    });

    // Build ResourceSeries, filtering capacities to those with ≥2 readings
    const resources: ResourceSeries[] = [];
    resourceMap.forEach(({ label, caps }, resourceId) => {
      const capacities: CapacityEntry[] = [];
      caps.forEach(({ data, rawData }, name) => {
        if (data.filter(v => v !== null).length < 2) {
          return;
        }
        const capDecl = getCapacityDecl(resourceId, name);
        const spec = capDecl ? quantitySpecs[capDecl.quantity] : undefined;
        const statLabel = spec?.symbol ? `${name} (${spec.symbol})` : name;
        const formatter: (v: number | bigint) => string =
          capDecl && spec ? v => formatQuantity(v, spec, capDecl.kind) : v => formatBytes(v);
        capacities.push({ name, statLabel, data, rawData, formatter });
      });
      if (capacities.length > 0) {
        if (defaultCapacityPredicate) {
          capacities.sort((a, b) => {
            const aPref = defaultCapacityPredicate(a.name) ? 0 : 1;
            const bPref = defaultCapacityPredicate(b.name) ? 0 : 1;
            return aPref - bPref;
          });
        }
        resources.push({ resourceId, label, capacities });
      }
    });

    if (defaultCapacityPredicate) {
      resources.sort((a, b) => {
        const aPref = a.capacities.some(c => defaultCapacityPredicate(c.name)) ? 0 : 1;
        const bPref = b.capacities.some(c => defaultCapacityPredicate(c.name)) ? 0 : 1;
        return aPref - bPref;
      });
    }

    return { resources, stateLabels };
  }, [transitions, resourceLabel, quantitySpecs, getCapacityDecl, defaultCapacityPredicate]);

  // Reset selections when the entity changes
  useEffect(() => {
    setSelectedResourceId(null);
    setSelectedCapacityName(null);
  }, [transitions]);

  // Resolve active resource
  const activeResource =
    resources.find(r => r.resourceId === selectedResourceId) ?? resources[0] ?? null;

  // Resolve active capacity — reset capacity selection when resource changes
  const activeCapacity =
    activeResource?.capacities.find(c => c.name === selectedCapacityName) ??
    activeResource?.capacities[0] ??
    null;

  const onEvents = useMemo(
    () => ({
      updateAxisPointer: (event: AxisPointerEvent) => {
        const value = event.axesInfo?.find(info => info.axisDim === 'x')?.value;
        setDataIndex(typeof value === 'number' ? Math.round(value) : null);
      },
    }),
    []
  );
  const hover = pointer && dataIndex != null ? { ...pointer, dataIndex } : null;
  const clearHover = () => {
    setPointer(null);
    setDataIndex(null);
  };

  const tooltipItems =
    hover && activeCapacity && activeResource
      ? (() => {
          const value = activeCapacity.data[hover.dataIndex];
          if (value == null) {
            return [];
          }
          const raw = activeCapacity.rawData[hover.dataIndex];
          return [
            {
              id: activeResource.label,
              label: activeResource.label,
              value: activeCapacity.formatter(raw ?? value),
            },
          ];
        })()
      : [];

  const option = useMemo(
    () => ({
      animation: false,
      grid: GRID,
      xAxis: {
        type: 'category' as const,
        data: stateLabels,
        boundaryGap: false,
        axisLabel: {
          show: true,
          fontSize: 9,
          interval: 0,
          formatter: (_val: string, idx: number) => String(idx + 1),
        },
        axisLine: { show: false },
        axisTick: { show: false },
      },
      yAxis: {
        type: 'value' as const,
        splitNumber: 3,
        axisLabel: {
          show: true,
          fontSize: 9,
          formatter: (v: number) =>
            activeCapacity ? activeCapacity.formatter(v) : formatBytes(v, 0),
        },
        splitLine: { show: true, lineStyle: { opacity: 0.25 } },
        minInterval: 1,
      },
      tooltip: {
        trigger: 'axis' as const,
        showContent: false,
        axisPointer: { type: 'line' as const, snap: true },
      },
      series: activeCapacity
        ? [
            {
              type: 'line' as const,
              name: activeResource?.label ?? '',
              data: activeCapacity.data,
              connectNulls: false,
              step: 'end' as const,
              symbol: 'circle',
              symbolSize: 5,
              lineStyle: { width: 1.5 },
            },
          ]
        : [],
    }),
    [activeCapacity, activeResource, stateLabels]
  );

  if (resources.length === 0) {
    return null;
  }

  return (
    <div className="shrink-0 border-b">
      <div className="flex items-center justify-between gap-2 px-2 py-1">
        <span className="font-mono text-[10px] text-muted-foreground">
          {activeCapacity?.statLabel}
        </span>
        <div className="flex items-center gap-1">
          {resources.length > 1 && (
            <SelectField
              ariaLabel="Select resource"
              options={resources.map(r => ({ value: r.resourceId, label: r.label }))}
              value={activeResource?.resourceId ?? ''}
              onValueChange={value => {
                if (!value) {
                  return;
                }
                setSelectedResourceId(value);
                setSelectedCapacityName(null);
              }}
              clearable={false}
              triggerClassName={SELECT_TRIGGER_CLASS}
            />
          )}
          {activeResource && activeResource.capacities.length > 1 && (
            <SelectField
              ariaLabel="Select capacity"
              options={activeResource.capacities.map(c => ({ value: c.name, label: c.name }))}
              value={activeCapacity?.name ?? ''}
              onValueChange={value => value && setSelectedCapacityName(value)}
              clearable={false}
              triggerClassName={SELECT_TRIGGER_CLASS}
            />
          )}
        </div>
      </div>
      <div
        onPointerMove={event => setPointer({ clientX: event.clientX, clientY: event.clientY })}
        onPointerLeave={clearHover}
        onPointerCancel={clearHover}
      >
        <EChartsReactCore
          echarts={echarts}
          theme={themeName}
          option={option}
          style={{ height: CHART_HEIGHT }}
          onChartReady={handleChartReady}
          onEvents={onEvents}
          autoResize={false}
          notMerge={false}
          lazyUpdate={false}
        />
        {hover && tooltipItems.length > 0 && (
          <PositionedTooltip clientX={hover.clientX} clientY={hover.clientY}>
            <FsmCapacityTooltip
              stateIndex={hover.dataIndex}
              stateName={transitions[hover.dataIndex]?.name ?? ''}
              items={tooltipItems}
            />
          </PositionedTooltip>
        )}
      </div>
    </div>
  );
}
