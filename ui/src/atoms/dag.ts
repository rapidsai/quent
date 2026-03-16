import { atom } from 'jotai';
import type { StatValue } from '@/services/query-plan/types';
export type { HoveredStatInfo } from '@/components/pivot-table/types';

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

import type { HoveredStatInfo } from '@/components/pivot-table/types';

/** Stat column being hovered in the table — drives DAG heatmap coloring */
export const hoveredStatAtom = atom<HoveredStatInfo | null>(null);

/** Operator type name being hovered in the table — highlights all DAG nodes of that type */
export const hoveredOperatorTypeAtom = atom<string | null>(null);

/** Set of node IDs to highlight (e.g. children of a hovered parent operator type) */
export const highlightedNodeIdsAtom = atom<Set<string> | null>(null);
