// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { atom } from 'jotai';
import type { OperatorSelectionState, SelectedOperatorGroupData } from '@quent/utils';
import { resolveOperatorSelectionCandidates } from '@quent/utils';
import {
  addOperatorSelection as addSelection,
  createEmptyOperatorSelectionState,
  getSelectedOperatorIds,
  removeOperatorSelection as removeSelection,
} from '../dag/operatorSelection';
import { findSelectedOperatorData } from '../dag/selectedOperatorData';
import {
  operatorSelectionCacheAtom,
  selectedOperatorsDataAtom,
  type CachedOperatorSelection,
} from './dagControls';

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
      selections: ReadonlyArray<{
        selectionId: string;
        label: string;
        operatorIds: Iterable<string>;
        selectedData?: SelectedOperatorGroupData;
      }>;
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

/** Canonical filtered operator IDs. */
export const selectedOperatorIdsAtom = atom<Set<string>>(new Set<string>());

/** Grouped filter metadata resolved from the canonical IDs and their payload cache. */
export const operatorSelectionAtom = atom<OperatorSelectionState>(get => {
  const selectedIds = get(selectedOperatorIdsAtom);
  const cache = get(operatorSelectionCacheAtom);
  const resolved = resolveOperatorSelectionCandidates(
    [...cache].map(([selectionId, selection]) => ({ selectionId, ...selection })),
    selectedIds
  );
  const selections = new Map(
    resolved.selections.map(({ selectionId, label, operatorIds }) => [
      selectionId,
      { label, operatorIds },
    ])
  );
  const selectedData = get(selectedOperatorsDataAtom);
  for (const operatorId of resolved.unresolvedOperatorIds) {
    selections.set(operatorId, {
      label: findSelectedOperatorData(selectedData, operatorId)?.label ?? operatorId,
      operatorIds: new Set([operatorId]),
    });
  }
  return { selections };
});

/** Updates operator filters and their selected details as one transaction. */
export const operatorSelectionActionAtom = atom(
  null,
  (get, set, action: OperatorSelectionAction): Set<string> => {
    const currentSelection = get(operatorSelectionAtom);
    const currentData = get(selectedOperatorsDataAtom);
    let nextSelection: OperatorSelectionState;
    let nextCache: Map<string, CachedOperatorSelection>;

    switch (action.type) {
      case 'add': {
        nextSelection = addSelection(
          currentSelection,
          action.selectionId,
          action.label,
          action.operatorIds
        );
        nextCache = new Map(
          [...nextSelection.selections].map(([selectionId, selection]) => [
            selectionId,
            {
              ...selection,
              data:
                selectionId === action.selectionId
                  ? action.selectedData
                  : findSelectedOperatorData(currentData, selectionId),
            },
          ])
        );
        break;
      }
      case 'remove': {
        nextSelection = removeSelection(currentSelection, action.selectionId);
        nextCache = new Map(
          [...nextSelection.selections].map(([selectionId, selection]) => [
            selectionId,
            {
              ...selection,
              data: findSelectedOperatorData(currentData, selectionId),
            },
          ])
        );
        break;
      }
      case 'replace': {
        nextSelection = createEmptyOperatorSelectionState();
        const replacementData = new Map<string, SelectedOperatorGroupData>();
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
            replacementData.set(selection.selectionId, selectedData);
          }
        }
        nextCache = new Map(
          [...nextSelection.selections].map(([selectionId, selection]) => [
            selectionId,
            {
              ...selection,
              data:
                replacementData.get(selectionId) ??
                findSelectedOperatorData(currentData, selectionId),
            },
          ])
        );
        break;
      }
      case 'hydrate': {
        nextSelection = currentSelection;
        nextCache = new Map(get(operatorSelectionCacheAtom));
        for (const selection of action.selections) {
          if (currentSelection.selections.has(selection.selectionId)) {
            nextCache.set(selection.selectionId, {
              label: selection.label,
              operatorIds: new Set(selection.operatorIds),
              data: selection.selectedData,
            });
          }
        }
        break;
      }
      case 'clear':
        nextSelection = createEmptyOperatorSelectionState();
        nextCache = new Map();
        break;
    }

    const nextIds = getSelectedOperatorIds(nextSelection);
    set(selectedOperatorIdsAtom, nextIds);
    set(operatorSelectionCacheAtom, nextCache);
    return nextIds;
  }
);

/** The currently selected plan ID in the query plan tree view */
export const selectedPlanIdAtom = atom<string>('');

/** Worker ID of the query plan tree item currently being hovered */
export const hoveredWorkerIdAtom = atom<string | null>(null);
