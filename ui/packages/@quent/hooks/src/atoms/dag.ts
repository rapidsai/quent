// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { atom } from 'jotai';
import type { InspectedNodeData, OperatorSelectionState } from '@quent/utils';
import {
  addOperatorSelection as addSelection,
  createOperatorSelectionState,
  createEmptyOperatorSelectionState,
  getActiveOperatorLabel,
  getLastOperatorSelectionId,
  getSelectedOperatorIds,
  removeOperatorSelection as removeSelection,
} from '../dag/operatorSelection';
import { removeInspectedNodeData, upsertInspectedNodeData } from '../dag/inspectedNodeData';
import { selectedNodesDataAtom } from './dagControls';

export type OperatorSelectionAction =
  | {
      type: 'add';
      selectionId: string;
      label: string;
      operatorIds: Iterable<string>;
      inspectedData: InspectedNodeData;
    }
  | { type: 'remove'; selectionId: string }
  | {
      type: 'replace';
      operatorIds: Iterable<string>;
      inspectedData?: InspectedNodeData;
    }
  | {
      type: 'hydrate';
      selections: ReadonlyArray<{
        selectionId: string;
        label: string;
        operatorIds: Iterable<string>;
        inspectedData: InspectedNodeData;
      }>;
      unresolvedOperatorIds: Iterable<string>;
    }
  | { type: 'clear' };

/** Canonical operator filter selection state */
export const operatorSelectionAtom = atom<OperatorSelectionState>(
  createEmptyOperatorSelectionState()
);

/** Updates operator filters and their inspected details as one transaction. */
export const operatorSelectionActionAtom = atom(
  null,
  (get, set, action: OperatorSelectionAction): Set<string> => {
    const currentSelection = get(operatorSelectionAtom);
    const currentData = get(selectedNodesDataAtom);
    let nextSelection: OperatorSelectionState;
    let nextData: ReadonlyMap<string, InspectedNodeData>;

    switch (action.type) {
      case 'add':
        nextSelection = addSelection(
          currentSelection,
          action.selectionId,
          action.label,
          action.operatorIds
        );
        nextData = new Map([...currentData].filter(([id]) => nextSelection.selections.has(id)));
        if (nextSelection.selections.has(action.selectionId)) {
          nextData = upsertInspectedNodeData(nextData, action.inspectedData);
        }
        break;
      case 'remove':
        nextSelection = removeSelection(currentSelection, action.selectionId);
        nextData = removeInspectedNodeData(currentData, action.selectionId);
        break;
      case 'replace': {
        const operatorIds = [...action.operatorIds];
        if (action.inspectedData) {
          nextSelection = addSelection(
            createEmptyOperatorSelectionState(),
            action.inspectedData.nodeId,
            action.inspectedData.label,
            operatorIds
          );
          nextData = new Map([[action.inspectedData.nodeId, action.inspectedData]]);
        } else {
          nextSelection = createOperatorSelectionState(operatorIds);
          nextData = new Map([...currentData].filter(([id]) => nextSelection.selections.has(id)));
        }
        break;
      }
      case 'hydrate': {
        nextSelection = createEmptyOperatorSelectionState();
        nextData = new Map();
        for (const id of action.unresolvedOperatorIds) {
          nextSelection = addSelection(nextSelection, id, id, [id]);
        }
        for (const selection of action.selections) {
          nextSelection = addSelection(
            nextSelection,
            selection.selectionId,
            selection.label,
            selection.operatorIds
          );
          if (nextSelection.selections.has(selection.selectionId)) {
            nextData = upsertInspectedNodeData(nextData, selection.inspectedData);
          }
        }
        break;
      }
      case 'clear':
        nextSelection = createEmptyOperatorSelectionState();
        nextData = new Map();
        break;
    }

    set(operatorSelectionAtom, nextSelection);
    set(selectedNodesDataAtom, nextData);
    return getSelectedOperatorIds(nextSelection);
  }
);

/** The operator IDs represented by the current selections */
export const selectedNodeIdsAtom = atom(
  get => getSelectedOperatorIds(get(operatorSelectionAtom)),
  (_get, set, operatorIds: Set<string>) =>
    set(operatorSelectionActionAtom, { type: 'replace', operatorIds })
);

/** Display label of the active operator selection */
export const selectedOperatorLabelAtom = atom(
  get => getActiveOperatorLabel(get(operatorSelectionAtom)),
  (get, set, label: string | null) => {
    const state = get(operatorSelectionAtom);
    if (label === null) {
      set(operatorSelectionAtom, { ...state, activeId: null });
      return;
    }

    const activeId = state.activeId ?? getLastOperatorSelectionId(state.selections);
    if (!activeId) {
      return;
    }
    const activeSelection = state.selections.get(activeId);
    if (!activeSelection) {
      return;
    }

    const selections = new Map(state.selections);
    selections.set(activeId, { ...activeSelection, label });
    set(operatorSelectionAtom, { selections, activeId });
  }
);

/** The currently selected plan ID in the query plan tree view */
export const selectedPlanIdAtom = atom<string>('');

/** Worker ID of the query plan tree item currently being hovered */
export const hoveredWorkerIdAtom = atom<string | null>(null);
