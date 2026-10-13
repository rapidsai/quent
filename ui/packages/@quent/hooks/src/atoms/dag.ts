// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { atom } from 'jotai';
import type {
  OperatorSelectionInput,
  OperatorSelectionState,
  SelectedOperatorGroupData,
} from '@quent/utils';
import {
  addOperatorSelection as addSelection,
  createEmptyOperatorSelectionState,
  getSelectedOperatorIds,
  removeOperatorSelection as removeSelection,
} from '../dag/operatorSelection';
import {
  findSelectedOperatorData,
  removeSelectedOperatorData,
  upsertSelectedOperatorData,
} from '../dag/selectedOperatorData';
import { selectedOperatorsDataAtom } from './dagControls';

export type OperatorSelectionAction =
  | {
      type: 'add';
      selectionId: string;
      label: string;
      operatorIds: Iterable<string>;
      selectedData: SelectedOperatorGroupData;
    }
  | { type: 'remove'; selectionId: string }
  | {
      type: 'replace';
      selections: ReadonlyArray<
        OperatorSelectionInput & { selectedData?: SelectedOperatorGroupData }
      >;
    }
  | {
      type: 'hydrate';
      selections: ReadonlyArray<{
        selectionId: string;
        label: string;
        operatorIds: Iterable<string>;
        selectedData: SelectedOperatorGroupData;
      }>;
    }
  | { type: 'clear' };

/** Canonical operator filter selection state */
export const operatorSelectionAtom = atom<OperatorSelectionState>(
  createEmptyOperatorSelectionState()
);

/** Updates operator filters and their selected details as one transaction. */
export const operatorSelectionActionAtom = atom(
  null,
  (get, set, action: OperatorSelectionAction): Set<string> => {
    const currentSelection = get(operatorSelectionAtom);
    const currentData = get(selectedOperatorsDataAtom);
    let nextSelection: OperatorSelectionState;
    let nextData: ReadonlyMap<string, SelectedOperatorGroupData>;

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
          nextData = upsertSelectedOperatorData(nextData, action.selectionId, action.selectedData);
        }
        break;
      case 'remove':
        nextSelection = removeSelection(currentSelection, action.selectionId);
        nextData = removeSelectedOperatorData(currentData, action.selectionId);
        break;
      case 'replace': {
        nextSelection = createEmptyOperatorSelectionState();
        nextData = new Map();
        for (const selection of action.selections) {
          nextSelection = addSelection(
            nextSelection,
            selection.selectionId,
            selection.label,
            selection.operatorIds
          );
          if (!nextSelection.selections.has(selection.selectionId)) {
            continue;
          }
          const selectedData =
            selection.selectedData ?? findSelectedOperatorData(currentData, selection.selectionId);
          if (selectedData) {
            nextData = upsertSelectedOperatorData(nextData, selection.selectionId, selectedData);
          }
        }
        nextData = new Map([...nextData].filter(([id]) => nextSelection.selections.has(id)));
        break;
      }
      case 'hydrate': {
        nextSelection = currentSelection;
        nextData = new Map(currentData);
        for (const selection of action.selections) {
          if (currentSelection.selections.has(selection.selectionId)) {
            nextData = upsertSelectedOperatorData(
              nextData,
              selection.selectionId,
              selection.selectedData
            );
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
    set(selectedOperatorsDataAtom, nextData);
    return getSelectedOperatorIds(nextSelection);
  }
);

/** The operator IDs represented by the current selections */
export const selectedOperatorIdsAtom = atom(get =>
  getSelectedOperatorIds(get(operatorSelectionAtom))
);

/** The currently selected plan ID in the query plan tree view */
export const selectedPlanIdAtom = atom<string>('');

/** Worker ID of the query plan tree item currently being hovered */
export const hoveredWorkerIdAtom = atom<string | null>(null);
