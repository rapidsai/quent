// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { DAGNode, DAGEdge, NodeColoring, EdgeWidthConfig, EdgeColoring } from './types';
import { parseCustomStatistics } from '@/lib/queryBundle.utils';
import { getActivePalette } from '@/services/colors';
import { formatDuration } from '@/services/formatters';

function formatBytes(value: number): string {
  const abs = Math.abs(value);
  const sign = value < 0 ? '-' : '';
  if (abs >= 1099511627776) return `${sign}${(abs / 1099511627776).toFixed(1)} TiB`;
  if (abs >= 1073741824) return `${sign}${(abs / 1073741824).toFixed(1)} GiB`;
  if (abs >= 1048576) return `${sign}${(abs / 1048576).toFixed(1)} MiB`;
  if (abs >= 1024) return `${sign}${(abs / 1024).toFixed(1)} KiB`;
  return `${sign}${Math.round(abs)} B`;
}

/**
 * Infer a display formatter for a DAG stat field based on its name.
 * Returns null when no special formatting applies — callers should fall back to String(value).
 */
export function inferFieldFormatter(fieldName: string): ((value: number) => string) | null {
  if (fieldName.endsWith('_ns')) return v => formatDuration(v / 1e6);
  if (fieldName.endsWith('_bytes') || fieldName === 'bytes') return formatBytes;
  if (fieldName.endsWith('_mbs')) return v => `${v.toFixed(1)} MB/s`;
  if (
    fieldName.endsWith('_ratio') ||
    fieldName.endsWith('_fraction') ||
    fieldName.endsWith('_fpr') ||
    fieldName.endsWith('_selectivity') ||
    fieldName.endsWith('_rate')
  )
    return v => `${(v * 100).toFixed(1)}%`;
  return null;
}

export function computeNodeColoring(nodes: DAGNode[], field: string | null): NodeColoring {
  if (!field || !nodes.length) return null;

  const entries = nodes.flatMap(node => {
    const stat = parseCustomStatistics(node.metadata?.rawNode).find(s => s.key === field);
    if (stat?.value == null) return [];
    return [{ id: node.id, value: stat.value }];
  });
  if (!entries.length) return null;

  if (entries.every(e => typeof e.value === 'number')) {
    const nums = entries.map(e => e.value as number);
    return {
      type: 'continuous',
      values: new Map(entries.map(e => [e.id, e.value as number])),
      min: Math.min(...nums),
      max: Math.max(...nums),
    };
  }

  const palette = getActivePalette();
  const uniqueValues = [...new Set(entries.map(e => String(e.value)))];
  const valueColor = new Map(uniqueValues.map((v, i) => [v, palette[i % palette.length]]));
  return {
    type: 'categorical',
    colorMap: new Map(entries.map(e => [e.id, valueColor.get(String(e.value))!])),
    categoryMap: valueColor,
  };
}

export function computeEdgeColoring(edges: DAGEdge[], field: string | null): EdgeColoring {
  if (!field || !edges.length) return null;

  const entries = edges.flatMap(edge => {
    const stat = (edge.portStats ?? []).find(s => s.key === field);
    if (stat?.value == null) return [];
    return [{ id: edge.id, value: stat.value }];
  });
  if (!entries.length) return null;

  if (entries.every(e => typeof e.value === 'number')) {
    const nums = entries.map(e => e.value as number);
    return {
      type: 'continuous',
      values: new Map(entries.map(e => [e.id, e.value as number])),
      min: Math.min(...nums),
      max: Math.max(...nums),
    };
  }

  const palette = getActivePalette();
  const uniqueValues = [...new Set(entries.map(e => String(e.value)))];
  const valueColor = new Map(uniqueValues.map((v, i) => [v, palette[i % palette.length]]));
  return {
    type: 'categorical',
    colorMap: new Map(entries.map(e => [e.id, valueColor.get(String(e.value))!])),
    labelMap: new Map(entries.map(e => [e.id, String(e.value)])),
    categoryMap: valueColor,
  };
}

export function formatMetricValue(v: number): string {
  if (Math.abs(v) >= 1e9) return `${(v / 1e9).toFixed(1)}B`;
  if (Math.abs(v) >= 1e6) return `${(v / 1e6).toFixed(1)}M`;
  if (Math.abs(v) >= 1e3) return `${(v / 1e3).toFixed(1)}K`;
  return Number.isInteger(v) ? String(v) : v.toFixed(2);
}

export function computeEdgeWidthConfig(edges: DAGEdge[], field: string | null): EdgeWidthConfig {
  if (!field || !edges.length) return null;

  const entries = edges.flatMap(edge => {
    const stat = (edge.portStats ?? []).find(s => s.key === field);
    if (typeof stat?.value !== 'number') return [];
    return [{ id: edge.id, value: stat.value }];
  });
  if (!entries.length) return null;

  const nums = entries.map(e => e.value);
  return {
    values: new Map(entries.map(e => [e.id, e.value])),
    min: Math.min(...nums),
    max: Math.max(...nums),
  };
}
