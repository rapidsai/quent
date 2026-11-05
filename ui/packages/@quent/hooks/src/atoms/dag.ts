// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { atom } from 'jotai';
import type { OperatorSelectionState, SelectedOperatorGroupData } from '@quent/utils';
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
import { requestDagDetailAttentionAtom } from './dagDetailAttention';

export type OperatorSelectionAction =
  | {
      type: 'add';
      selectionId: string;
      label: string;
      operatorIds: Iterable<string>;
      selectedData: SelectedOperatorGroupData;
      detailAttention?: 'show' | 'preserve' | 'none';
    }
  | { type: 'remove'; selectionId: string }
  | {
      type: 'replace';
      selections: ReadonlyArray<{
        selectionId: string;
        label: string;
        operatorIds: Iterable<string>;
        selectedData?: SelectedOperatorGroupData;
      }>;
      detailAttention?: 'show' | 'preserve' | 'none';
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

/** Canonical operator filter selection state. */
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
      case 'add': {
        nextSelection = addSelection(
          currentSelection,
          action.selectionId,
          action.label,
          action.operatorIds
        );
        nextData = new Map(
          [...currentData].filter(([selectionId]) => nextSelection.selections.has(selectionId))
        );
        if (nextSelection.selections.has(action.selectionId)) {
          nextData = upsertSelectedOperatorData(nextData, action.selectionId, action.selectedData);
        }
        break;
      }
      case 'remove': {
        nextSelection = removeSelection(currentSelection, action.selectionId);
        nextData = removeSelectedOperatorData(currentData, action.selectionId);
        break;
      }
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
          const selectedData =
            selection.selectedData ?? findSelectedOperatorData(currentData, selection.selectionId);
          if (selectedData) {
            nextData = upsertSelectedOperatorData(nextData, selection.selectionId, selectedData);
          }
        }
        nextData = new Map(
          [...nextData].filter(([selectionId]) => nextSelection.selections.has(selectionId))
        );
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
    if ((action.type === 'add' || action.type === 'replace') && action.detailAttention !== 'none') {
      let addedSelectionId: string | undefined;
      for (const selectionId of nextSelection.selections.keys()) {
        if (!currentSelection.selections.has(selectionId)) {
          addedSelectionId = selectionId;
        }
      }
      if (addedSelectionId) {
        set(requestDagDetailAttentionAtom, {
          kind: 'operator',
          id: addedSelectionId,
          activateTab: action.detailAttention !== 'preserve',
        });
      }
    }
    return getSelectedOperatorIds(nextSelection);
  }
);

/** The operator IDs represented by the current filters. */
export const selectedOperatorIdsAtom = atom(get =>
  getSelectedOperatorIds(get(operatorSelectionAtom))
);

/** The currently selected plan ID in the query plan tree view */
export const selectedPlanIdAtom = atom<string>('');

/** Worker ID of the query plan tree item currently being hovered */
export const hoveredWorkerIdAtom = atom<string | null>(null);
