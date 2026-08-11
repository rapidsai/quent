// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo } from 'react';
import EChartsReactCore from 'echarts-for-react/lib/core';
import type { FsmTransition } from '@quent/utils';
import { formatBytes } from '@quent/utils';
import { echarts } from '../lib/echarts';
import { useChartResize } from '../lib/useChartResize';
import { useTimelineEchartsTheme } from '../timeline/timelineEchartsTheme';

const CHART_HEIGHT = 90;

function isBytesStat(name: string): boolean {
  return (
    name.includes('_bytes') ||
    name.endsWith('_byte') ||
    name.startsWith('bytes_') ||
    name === 'bytes'
  );
}

interface CapacitySeries {
  label: string;
  // Full-length array aligned to transitions — null where no reading exists
  data: Array<number | null>;
}

export interface FsmCapacityChartProps {
  transitions: FsmTransition[];
  isDark: boolean;
  resourceLabel: (id: string) => string;
}

export function FsmCapacityChart({ transitions, isDark, resourceLabel }: FsmCapacityChartProps) {
  const { themeName } = useTimelineEchartsTheme(isDark);
  const { handleChartReady } = useChartResize();

  const { series, stateLabels } = useMemo(() => {
    const n = transitions.length;
    const stateLabels = transitions.map((t, i) => `${i + 1}. ${t.name}`);

    // Build per-resource full-length arrays (null = no reading at that state)
    const dataMap = new Map<string, Array<number | null>>();
    const labelMap = new Map<string, string>();

    transitions.forEach((t, i) => {
      t.usages.forEach(usage => {
        const resourceName = resourceLabel(usage.resource);
        usage.capacities.forEach(([name, cap]) => {
          if (cap == null || !isBytesStat(name)) return;
          const key = `${usage.resource} ${name}`;
          if (!dataMap.has(key)) {
            dataMap.set(key, Array<number | null>(n).fill(null));
            labelMap.set(key, name === 'capacity_bytes' ? resourceName : `${resourceName} ${name}`);
          }
          dataMap.get(key)![i] = Number(cap);
        });
      });
    });

    // Only show resources with readings in at least 2 states
    const series: CapacitySeries[] = [...dataMap.entries()]
      .filter(([, data]) => data.filter(v => v !== null).length >= 2)
      .map(([key, data]) => ({ label: labelMap.get(key) ?? key, data }));

    return { series, stateLabels };
  }, [transitions, resourceLabel]);

  const option = useMemo(
    () => ({
      animation: false,
      grid: { left: 52, right: 8, top: 8, bottom: 36 },
      xAxis: {
        type: 'category' as const,
        data: stateLabels,
        boundaryGap: false,
        axisLabel: {
          show: true,
          fontSize: 9,
          interval: 0,
          // Show only the state number to save space; full name is in the tooltip
          formatter: (_val: string, idx: number) => String(idx + 1),
        },
        axisLine: { show: false },
        axisTick: { show: false },
      },
      yAxis: {
        type: 'value' as const,
        axisLabel: {
          show: true,
          fontSize: 9,
          formatter: (v: number) => formatBytes(v, 0),
        },
        splitLine: { show: true, lineStyle: { opacity: 0.25 } },
        minInterval: 1,
      },
      tooltip: {
        trigger: 'axis' as const,
        formatter: (
          params: Array<{ seriesName: string; value: number | null; dataIndex: number }>
        ) => {
          const idx = params[0]?.dataIndex ?? 0;
          const stateName = transitions[idx]?.name ?? '';
          const lines = params
            .filter(p => p.value != null)
            .map(p => `${p.seriesName}: ${formatBytes(p.value!)}`);
          if (lines.length === 0) return '';
          return [`<strong>${idx + 1}. ${stateName}</strong>`, ...lines].join('<br/>');
        },
      },
      series: series.map(s => ({
        type: 'line' as const,
        name: s.label,
        data: s.data,
        connectNulls: false,
        step: 'end' as const,
        symbol: 'circle',
        symbolSize: 5,
        lineStyle: { width: 1.5 },
      })),
    }),
    [series, stateLabels, transitions]
  );

  if (series.length === 0) return null;

  return (
    <div className="shrink-0 border-b">
      <EChartsReactCore
        echarts={echarts}
        theme={themeName}
        option={option}
        style={{ height: CHART_HEIGHT }}
        onChartReady={handleChartReady}
        autoResize={false}
        notMerge={false}
        lazyUpdate={false}
      />
    </div>
  );
}
