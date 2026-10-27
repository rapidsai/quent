// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createStore } from 'jotai';
import { describe, expect, it } from 'vitest';
import {
  displayedOperatorDetailsAtom,
  operatorDetailsActionAtom,
  pinnedOperatorDetailsAtom,
} from '../atoms/operatorDetails';
import { selectedOperatorsDataAtom } from '../atoms/dagControls';

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
  it('replaces, toggles, and clears pins', () => {
    const store = createStore();
    store.set(operatorDetailsActionAtom, {
      type: 'replace',
      entries: [{ selectionId: 'scan', data: scan }],
    });
    store.set(operatorDetailsActionAtom, {
      type: 'toggle',
      entry: { selectionId: 'join', data: join },
    });
    expect([...store.get(pinnedOperatorDetailsAtom).keys()]).toEqual(['scan', 'join']);

    store.set(operatorDetailsActionAtom, {
      type: 'toggle',
      entry: { selectionId: 'scan', data: scan },
    });
    expect([...store.get(pinnedOperatorDetailsAtom).keys()]).toEqual(['join']);

    expect(store.get(displayedOperatorDetailsAtom)).toEqual([{ selectionId: 'join', data: join }]);

    store.set(operatorDetailsActionAtom, { type: 'clear' });
    expect(store.get(displayedOperatorDetailsAtom)).toEqual([]);
  });

  it('hydrates IDs restored before DAG data becomes available', () => {
    const store = createStore();
    store.set(operatorDetailsActionAtom, {
      type: 'replaceIds',
      selectionIds: ['scan', 'missing'],
    });
    expect(store.get(displayedOperatorDetailsAtom)).toEqual([]);

    store.set(operatorDetailsActionAtom, {
      type: 'hydrate',
      entries: [
        { selectionId: 'scan', data: scan },
        { selectionId: 'join', data: join },
      ],
    });
    expect(store.get(displayedOperatorDetailsAtom)).toEqual([{ selectionId: 'scan', data: scan }]);
    expect([...store.get(pinnedOperatorDetailsAtom).keys()]).toEqual(['scan', 'missing']);
  });

  it('pins and unpins one operator without replacing the others', () => {
    const store = createStore();
    store.set(operatorDetailsActionAtom, {
      type: 'pin',
      entry: { selectionId: 'scan', data: scan },
    });
    store.set(operatorDetailsActionAtom, {
      type: 'pin',
      entry: { selectionId: 'join', data: join },
    });
    store.set(operatorDetailsActionAtom, { type: 'unpin', selectionId: 'scan' });
    expect(store.get(displayedOperatorDetailsAtom)).toEqual([{ selectionId: 'join', data: join }]);
  });

  it('combines pins and filters without duplicating an operator', () => {
    const store = createStore();
    store.set(
      selectedOperatorsDataAtom,
      new Map([
        ['filtered-scan', scan],
        ['join', join],
      ])
    );
    const pinnedScan = { ...scan, label: 'Pinned Scan' };
    store.set(operatorDetailsActionAtom, {
      type: 'pin',
      entry: { selectionId: 'dag-scan', data: pinnedScan },
    });

    expect(store.get(displayedOperatorDetailsAtom)).toEqual([
      { selectionId: 'dag-scan', data: pinnedScan },
      { selectionId: 'join', data: join },
    ]);

    store.set(operatorDetailsActionAtom, { type: 'clear' });
    expect(store.get(displayedOperatorDetailsAtom)).toEqual([
      { selectionId: 'filtered-scan', data: scan },
      { selectionId: 'join', data: join },
    ]);
  });
});
