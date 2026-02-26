import { atom } from 'jotai';

/** The set of currently selected node IDs in the DAG chart */
export const selectedNodeIdsAtom = atom(new Set<string>());

/** Display label of the currently selected operator (set alongside selectedNodeIdsAtom) */
export const selectedOperatorLabelAtom = atom<string | null>(null);

/** The currently selected plan ID in the query plan tree view */
export const selectedPlanIdAtom = atom<string>('');
