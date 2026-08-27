// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { DynamicValue } from './types/index';

// Pure-data types for DAG coloring, width configuration, and node/edge shapes.
// These are kept in @quent/utils to avoid circular dependencies between
// @quent/hooks (which holds DAG atoms) and @quent/components (which holds DAG rendering).

export type ContinuousNodeColoring = {
  type: 'continuous';
  values: Map<string, number>; // operatorId → numeric value
  min: number;
  max: number;
};

export type CategoricalNodeColoring = {
  type: 'categorical';
  colorMap: Map<string, string>; // operatorId → hex color
  categoryMap: Map<string, string>; // category value → hex color
};

export type NodeColoring = ContinuousNodeColoring | CategoricalNodeColoring | null;

export type EdgeWidthConfig = {
  values: Map<string, number>; // edgeId → numeric value
  min: number;
  max: number;
} | null;

export type ContinuousEdgeColoring = {
  type: 'continuous';
  values: Map<string, number>; // edgeId → numeric value
  min: number;
  max: number;
};

export type CategoricalEdgeColoring = {
  type: 'categorical';
  colorMap: Map<string, string>; // edgeId → hex color
  labelMap: Map<string, string>; // edgeId → raw value string
  categoryMap: Map<string, string>; // category value → hex color
};

export type EdgeColoring = ContinuousEdgeColoring | CategoricalEdgeColoring | null;

export const NODE_LABEL_FIELD = {
  NAME: 'name',
  ID: 'id',
  TYPE: 'type',
} as const;

export type NodeLabelField = (typeof NODE_LABEL_FIELD)[keyof typeof NODE_LABEL_FIELD];

export const DAG_LAYOUT_DIRECTION = {
  /** Sources (e.g. Scan) at the bottom, flowing up toward the result. Conventional for query profiling. */
  BOTTOM_TO_TOP: 'bottom-to-top',
  /** Sources at the top, flowing down toward the result. */
  TOP_TO_BOTTOM: 'top-to-bottom',
} as const;

export type DagLayoutDirection = (typeof DAG_LAYOUT_DIRECTION)[keyof typeof DAG_LAYOUT_DIRECTION];

export type StatValue = DynamicValue | null;

export interface DAGNode {
  id: string;
  label: string;
  type: string;
  metadata?: {
    details?: string;
    estimates?: unknown[];
    identifier?: string;
    rawNode?: unknown;
    stageId?: string;
    [key: string]: unknown;
  };
}

export interface DAGEdge {
  id: string;
  source: string;
  target: string;
  type?: 'smoothstep' | 'default' | 'straight';
  portStats?: Array<{ key: string; value: StatValue }>; // from source port
}
