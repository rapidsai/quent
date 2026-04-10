// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { atom } from 'jotai';
import type { StatValue } from '@/services/query-plan/types';
import { HoveredStatInfo } from '@/components/pivot-table/types';
export type { HoveredStatInfo } from '@/components/pivot-table/types';
import type { NodeColoring, EdgeWidthConfig, EdgeColoring } from '@/services/query-plan/types';
import type { ContinuousPaletteName } from '@/services/colors';

export interface HoveredOperatorInfo {
  nodeId: string;
  label: string;
  operationType: string;
  stats: Array<{ key: string; value: StatValue }>;
}

/** The set of currently selected node IDs in the DAG chart */
export const selectedNodeIdsAtom = atom(new Set<string>());

/** Display label of the currently selected operator (set alongside selectedNodeIdsAtom) */
export const selectedOperatorLabelAtom = atom<string | null>(null);

/** The currently selected plan ID in the query plan tree view */
export const selectedPlanIdAtom = atom<string>('');

/** Worker ID of the query plan tree item currently being hovered */
export const hoveredWorkerIdAtom = atom<string | null>(null);

/** Operator ID currently being hovered (shared between DAG and table) */
export const hoveredOperatorIdAtom = atom<string | null>(null);

/** Full info for the operator being hovered in the DAG (drives the stats overlay) */
export const hoveredOperatorInfoAtom = atom<HoveredOperatorInfo | null>(null);

/** Stat column being hovered in the table — drives DAG heatmap coloring */
export const hoveredStatAtom = atom<HoveredStatInfo | null>(null);

/** Operator type name being hovered in the table — highlights all DAG nodes of that type */
export const hoveredOperatorTypeAtom = atom<string | null>(null);

/** Set of node IDs to highlight (e.g. children of a hovered parent operator type) */
export const highlightedNodeIdsAtom = atom<Set<string> | null>(null);

/** Field to color each DAG node by */
export const selectedColorField = atom<string | null>(null);

/** Computed node coloring config (written by QueryPlan, read by QueryPlanNode) */
export const nodeColoringAtom = atom<NodeColoring>(null);

/** Field to scale edge widths by */
export const selectedEdgeWidthFieldAtom = atom<string | null>(null);

/** Computed edge width config (written by QueryPlan, read by VariableWidthEdge) */
export const edgeWidthConfigAtom = atom<EdgeWidthConfig>(null);

/** Field to color each DAG edge by */
export const selectedEdgeColorFieldAtom = atom<string | null>(null);

/** Computed edge coloring config (written by QueryPlan, read by VariableWidthEdge) */
export const edgeColoringAtom = atom<EdgeColoring>(null);

/** Which field to use as the primary label on each DAG node */
export const NODE_LABEL_FIELD = {
  NAME: 'name',
  ID: 'id',
  TYPE: 'type',
} as const;
export type NodeLabelField = (typeof NODE_LABEL_FIELD)[keyof typeof NODE_LABEL_FIELD];
export const selectedNodeLabelFieldAtom = atom<NodeLabelField>(NODE_LABEL_FIELD.NAME);

/** Continuous color palette used for node coloring */
export const nodeColorPaletteAtom = atom<ContinuousPaletteName>('blue');

/** Continuous color palette used for edge coloring */
export const edgeColorPaletteAtom = atom<ContinuousPaletteName>('teal');
