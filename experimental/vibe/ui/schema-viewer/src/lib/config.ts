// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type {
  EntityGraphConfig,
  ResolvedEntityGraphConfig,
} from './types';

export const DEFAULT_ENTITY_GRAPH_CONFIG: Readonly<ResolvedEntityGraphConfig> = {
  direction: 'down',
  edgeRouting: 'orthogonal',
  density: 'compact',
  layeringStrategy: 'network-simplex',
  nodePlacementStrategy: 'linear-segments',
  hierarchicalGreedySwitch: false,
  layoutThoroughness: 7,
  highDegreeNodeTreatment: false,
  groupNamespaces: true,
  references: 'all',
  referenceLabels: 'interaction',
  showNodeMetadata: true,
  showViewSwitcher: true,
  fitPadding: 0.12,
  minZoom: 0.15,
  maxZoom: 3,
  nodeWidth: 208,
  nodeHeight: 76,
};

export function resolveEntityGraphConfig(
  config: EntityGraphConfig | null | undefined,
): ResolvedEntityGraphConfig {
  const minZoom = positive(config?.minZoom, DEFAULT_ENTITY_GRAPH_CONFIG.minZoom);
  const maxZoom = Math.max(
    minZoom,
    positive(config?.maxZoom, DEFAULT_ENTITY_GRAPH_CONFIG.maxZoom),
  );
  return {
    ...DEFAULT_ENTITY_GRAPH_CONFIG,
    ...config,
    fitPadding: nonNegative(
      config?.fitPadding,
      DEFAULT_ENTITY_GRAPH_CONFIG.fitPadding,
    ),
    layoutThoroughness: boundedInteger(
      config?.layoutThoroughness,
      DEFAULT_ENTITY_GRAPH_CONFIG.layoutThoroughness,
      1,
      100,
    ),
    minZoom,
    maxZoom,
    nodeWidth: positive(
      config?.nodeWidth,
      DEFAULT_ENTITY_GRAPH_CONFIG.nodeWidth,
    ),
    nodeHeight: positive(
      config?.nodeHeight,
      DEFAULT_ENTITY_GRAPH_CONFIG.nodeHeight,
    ),
  };
}

function positive(value: number | undefined, fallback: number): number {
  return value !== undefined && Number.isFinite(value) && value > 0
    ? value
    : fallback;
}

function nonNegative(value: number | undefined, fallback: number): number {
  return value !== undefined && Number.isFinite(value) && value >= 0
    ? value
    : fallback;
}

function boundedInteger(
  value: number | undefined,
  fallback: number,
  minimum: number,
  maximum: number,
): number {
  return value !== undefined && Number.isFinite(value)
    ? Math.min(maximum, Math.max(minimum, Math.round(value)))
    : fallback;
}
