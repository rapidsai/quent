// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// PRIVATE to @quent/hooks — do not export raw atoms (HOOKS-02).
// Consumers use the selector hooks exported from @quent/hooks index.ts.

import { atom } from 'jotai';
import type {
  NodeColoring,
  EdgeWidthConfig,
  EdgeColoring,
  NodeLabelField,
  DagLayoutDirection,
  SelectedOperatorGroupData,
  AggMode,
} from '@quent/utils';
import { NODE_LABEL_FIELD, DAG_LAYOUT_DIRECTION } from '@quent/utils';
import type { ContinuousPaletteName } from '@quent/utils';

/**
 * Stat-driven hover info shared between the pivot table and the DAG. Defined
 * here so `@quent/hooks` remains self-contained — both the DAG hooks below
 * and the pivot-table package consume this type.
 */
export interface HoveredStatInfo {
  name: string;
  /** item ID → numeric value for this stat */
  values: Map<string, number>;
  min: number;
  max: number;
  /** How to combine related items' values for a node with no entry of its own. */
  aggMode: AggMode;
}

export interface HighlightedNodeIdsState {
  hoveredStat: HoveredStatInfo | null;
  ids: Set<string> | null;
  source: 'dag' | 'table' | null;
  primaryOperatorId: string | null;
}

/** Details for every selected operator group, keyed by selection id. */
export const selectedOperatorsDataAtom = atom<ReadonlyMap<string, SelectedOperatorGroupData>>(
  new Map()
);

/** Consolidated hover/highlight state shared between table and DAG. */
export const highlightedNodeIdsAtom = atom<HighlightedNodeIdsState>({
  hoveredStat: null,
  ids: null,
  source: null,
  primaryOperatorId: null,
});

/** Stat column being hovered in the table — drives DAG heatmap coloring */
export const hoveredStatAtom = atom(
  get => get(highlightedNodeIdsAtom).hoveredStat,
  (get, set, value: HoveredStatInfo | null) => {
    set(highlightedNodeIdsAtom, { ...get(highlightedNodeIdsAtom), hoveredStat: value });
  }
);

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
export const selectedNodeLabelFieldAtom = atom<NodeLabelField>(NODE_LABEL_FIELD.NAME);

/** Continuous color palette used for node coloring */
export const nodeColorPaletteAtom = atom<ContinuousPaletteName>('blue');

/** Continuous color palette used for edge coloring */
export const edgeColorPaletteAtom = atom<ContinuousPaletteName>('teal');

/** Direction the DAG layout flows — defaults to sources at the bottom, result at the top */
export const selectedDagLayoutDirectionAtom = atom<DagLayoutDirection>(
  DAG_LAYOUT_DIRECTION.BOTTOM_TO_TOP
);
