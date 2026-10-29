// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { atom } from 'jotai';
import type { SelectedOperatorGroupData } from '@quent/utils';
import { selectedOperatorsDataAtom } from './dagControls';

export interface OperatorDetailEntry {
  selectionId: string;
  data: SelectedOperatorGroupData;
}

export type OperatorDetailsAction =
  | { type: 'pin'; entry: OperatorDetailEntry }
  | { type: 'unpin'; selectionId: string }
  | { type: 'toggle'; entry: OperatorDetailEntry }
  | { type: 'replace'; entries: readonly OperatorDetailEntry[] }
  | { type: 'replaceIds'; selectionIds: readonly string[] }
  | { type: 'hydrate'; entries: readonly OperatorDetailEntry[] }
  | { type: 'clear' };

/** Pinned operator details keyed by DAG selection id; undefined values await DAG hydration. */
export const pinnedOperatorDetailsAtom = atom<
  ReadonlyMap<string, SelectedOperatorGroupData | undefined>
>(new Map());

export const pinnedOperatorDetailIdsAtom = atom(
  get => new Set(get(pinnedOperatorDetailsAtom).keys())
);

export const displayedOperatorDetailsAtom = atom<OperatorDetailEntry[]>(get => {
  const details = new Map<string, SelectedOperatorGroupData>();
  const nodeIds = new Set<string>();
  for (const [selectionId, data] of get(pinnedOperatorDetailsAtom)) {
    if (data && !nodeIds.has(data.nodeId)) {
      details.set(selectionId, data);
      nodeIds.add(data.nodeId);
    }
  }
  for (const [selectionId, data] of get(selectedOperatorsDataAtom)) {
    if (!details.has(selectionId) && !nodeIds.has(data.nodeId)) {
      details.set(selectionId, data);
      nodeIds.add(data.nodeId);
    }
  }
  return [...details].map(([selectionId, data]) => ({ selectionId, data }));
});

export const operatorDetailsActionAtom = atom(
  null,
  (get, set, action: OperatorDetailsAction): ReadonlySet<string> => {
    const current = get(pinnedOperatorDetailsAtom);
    let next = new Map(current);

    switch (action.type) {
      case 'pin':
        next.set(action.entry.selectionId, action.entry.data);
        break;
      case 'unpin':
        next.delete(action.selectionId);
        break;
      case 'toggle':
        if (next.has(action.entry.selectionId)) {
          next.delete(action.entry.selectionId);
        } else {
          next.set(action.entry.selectionId, action.entry.data);
        }
        break;
      case 'replace':
        next = new Map(action.entries.map(entry => [entry.selectionId, entry.data]));
        break;
      case 'replaceIds':
        next = new Map(action.selectionIds.map(id => [id, current.get(id)]));
        break;
      case 'hydrate':
        for (const entry of action.entries) {
          if (next.has(entry.selectionId)) {
            next.set(entry.selectionId, entry.data);
          }
        }
        break;
      case 'clear':
        next.clear();
        break;
    }

    for (const [selectionId, data] of get(selectedOperatorsDataAtom)) {
      next.set(selectionId, data);
    }
    set(pinnedOperatorDetailsAtom, next);
    return new Set(next.keys());
  }
);
