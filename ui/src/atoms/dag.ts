import { atom } from 'jotai';
import type { NodeColoring, EdgeWidthConfig, EdgeColoring } from '@/services/query-plan/types';

/** The set of currently selected node IDs in the DAG chart */
export const selectedNodeIdsAtom = atom(new Set<string>());

/** Display label of the currently selected operator (set alongside selectedNodeIdsAtom) */
export const selectedOperatorLabelAtom = atom<string | null>(null);

/** The currently selected plan ID in the query plan tree view */
export const selectedPlanIdAtom = atom<string>('');

/** Worker ID of the query plan tree item currently being hovered */
export const hoveredWorkerIdAtom = atom<string | null>(null);

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
