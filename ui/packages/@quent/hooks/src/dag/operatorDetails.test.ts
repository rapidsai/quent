// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createStore } from 'jotai';
import { describe, expect, it } from 'vitest';
import { operatorSelectionActionAtom } from '../atoms/dag';
import { displayedOperatorDetailsAtom } from '../atoms/operatorDetails';

const scan = {
  nodeId: 'scan',
  label: 'Scan',
  operationType: 'scan',
  statistics: [],
};
const join = {
  nodeId: 'join',
  label: 'Join',
  operationType: 'join',
  statistics: [],
};

describe('operator details', () => {
  it('derives details from filtered operators', () => {
    const store = createStore();
    store.set(operatorSelectionActionAtom, {
      type: 'add',
      selectionId: 'scan',
      label: 'Scan',
      operatorIds: ['scan'],
      selectedData: scan,
    });
    store.set(operatorSelectionActionAtom, {
      type: 'add',
      selectionId: 'join',
      label: 'Join',
      operatorIds: ['join'],
      selectedData: join,
    });
    expect(store.get(displayedOperatorDetailsAtom)).toEqual([
      { selectionId: 'scan', data: scan },
      { selectionId: 'join', data: join },
    ]);
  });

  it('keeps cached details while the filtered operator is absent from the displayed plan', () => {
    const store = createStore();
    store.set(operatorSelectionActionAtom, {
      type: 'add',
      selectionId: 'scan',
      label: 'Scan',
      operatorIds: ['scan'],
      selectedData: scan,
    });
    store.set(operatorSelectionActionAtom, { type: 'hydrate', selections: [] });
    expect(store.get(displayedOperatorDetailsAtom)).toEqual([{ selectionId: 'scan', data: scan }]);
  });
});
