// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// PRIVATE to @quent/hooks — do not export raw atoms (HOOKS-02).
// Consumers use the selector hooks exported from @quent/hooks index.ts.

import { atom } from 'jotai';
import type {
  NodeColoring,
  EdgeWidthConfig,
  EdgeColoring,
  NodeLabelField,
  StatValue,
} from '@quent/utils';
import { NODE_LABEL_FIELD } from '@quent/utils';
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
}

export interface HighlightedNodeIdsState {
  hoveredStat: HoveredStatInfo | null;
  ids: Set<string> | null;
  source: 'dag' | 'table' | null;
  primaryOperatorId: string | null;
}

export interface InspectedNodeData {
  nodeId: string;
  label: string;
  operationType: string;
  statistics: Array<{ key: string; value: StatValue }>;
}

/** Data for the node currently being hovered (drives the preview panel) */
export const hoveredNodeDataAtom = atom<InspectedNodeData | null>(null);

/** Data for the currently selected/pinned node (persists in the panel after click) */
export const selectedNodeDataAtom = atom<InspectedNodeData | null>(null);

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

/**
 * IDs of operator nodes currently rendered in the DAG chart. Written by
 * `DAGChart` whenever its data changes; consumed by the effective
 * highlight/heatmap atoms below to decide whether a hover-driven dim is
 * actually meaningful.
 */
export const dagDisplayedNodeIdsAtom = atom<Set<string>>(new Set<string>());

function intersectsDisplayed(ids: Iterable<string>, displayed: Set<string>): boolean {
  for (const id of ids) {
    if (displayed.has(id)) return true;
  }
  return false;
}

/**
 * Highlight state for DAG consumers. Behaves like `highlightedNodeIdsAtom`,
 * except that when the highlight set has zero overlap with the nodes
 * currently shown in the DAG, `ids` is cleared. This prevents the DAG from
 * dimming everything just because (e.g.) a table-driven hover refers to
 * operators in a different plan.
 */
export const effectiveHighlightedNodeIdsAtom = atom<HighlightedNodeIdsState>(get => {
  const state = get(highlightedNodeIdsAtom);
  if (state.ids === null) return state;
  const displayed = get(dagDisplayedNodeIdsAtom);
  // Until the DAG has reported what it shows, fall back to the source state
  // so behavior is unchanged on first render.
  if (displayed.size === 0) return state;
  if (intersectsDisplayed(state.ids, displayed)) return state;
  return { ...state, ids: null, source: null, primaryOperatorId: null };
});

/**
 * Heatmap-driving stat hover for DAG consumers. Cleared when none of the
 * stat's per-operator values are present in the displayed DAG, so the chart
 * doesn't go fully muted on a hover that affects no visible node.
 */
export const effectiveHoveredStatAtom = atom<HoveredStatInfo | null>(get => {
  const stat = get(highlightedNodeIdsAtom).hoveredStat;
  if (!stat) return null;
  const displayed = get(dagDisplayedNodeIdsAtom);
  if (displayed.size === 0) return stat;
  if (intersectsDisplayed(stat.values.keys(), displayed)) return stat;
  return null;
});

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
