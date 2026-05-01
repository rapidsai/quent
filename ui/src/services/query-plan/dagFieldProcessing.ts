// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { DAGNode, DAGEdge, NodeColoring, EdgeWidthConfig, EdgeColoring } from './types';
import { parseCustomStatistics } from '@/lib/queryBundle.utils';
import { getActivePalette, type PaletteTheme } from '@/services/colors';

export function computeNodeColoring(
  nodes: DAGNode[],
  field: string | null,
  theme: PaletteTheme
): NodeColoring {
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

  const palette = getActivePalette(theme);
  const uniqueValues = [...new Set(entries.map(e => String(e.value)))];
  const valueColor = new Map(uniqueValues.map((v, i) => [v, palette[i % palette.length]]));
  return {
    type: 'categorical',
    colorMap: new Map(entries.map(e => [e.id, valueColor.get(String(e.value))!])),
    categoryMap: valueColor,
  };
}

export function computeEdgeColoring(
  edges: DAGEdge[],
  field: string | null,
  theme: PaletteTheme
): EdgeColoring {
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

  const palette = getActivePalette(theme);
  const uniqueValues = [...new Set(entries.map(e => String(e.value)))];
  const valueColor = new Map(uniqueValues.map((v, i) => [v, palette[i % palette.length]]));
  return {
    type: 'categorical',
    colorMap: new Map(entries.map(e => [e.id, valueColor.get(String(e.value))!])),
    labelMap: new Map(entries.map(e => [e.id, String(e.value)])),
    categoryMap: valueColor,
  };
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
