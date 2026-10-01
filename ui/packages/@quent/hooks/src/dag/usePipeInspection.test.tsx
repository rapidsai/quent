// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createStore, Provider } from 'jotai';
import { act, render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { inspectedPipeAtom, inspectedPipeRefAtom } from '../atoms/pipeInspection';
import { operatorSelectionActionAtom, selectedOperatorIdsAtom } from '../atoms/dag';
import { useSyncDisplayedPipes } from './usePipeInspection';
import type { DAGEdge } from '@quent/utils';

const one: DAGEdge = {
  id: 'one',
  source: 's',
  target: 't',
  sourcePortId: 'out-1',
  targetPortId: 'in-1',
};
const two: DAGEdge = {
  id: 'two',
  source: 's',
  target: 't',
  sourcePortId: 'out-2',
  targetPortId: 'in-2',
};
function Sync({ edges }: { edges: DAGEdge[] }) {
  useSyncDisplayedPipes(edges);
  return null;
}

describe('pipe inspection', () => {
  it('resolves parallel pipes independently and refreshes evidence without changing operator filters', () => {
    const store = createStore();
    store.set(operatorSelectionActionAtom, {
      type: 'replace',
      selections: [{ selectionId: 's', label: 'Scan', operatorIds: new Set(['s']) }],
    });
    const view = render(
      <Provider store={store}>
        <Sync edges={[one, two]} />
      </Provider>
    );
    act(() => {
      store.set(inspectedPipeRefAtom, { sourcePortId: 'out-2', targetPortId: 'in-2' });
    });
    expect(store.get(inspectedPipeAtom)?.id).toBe('two');
    expect(store.get(selectedOperatorIdsAtom)).toEqual(new Set(['s']));
    view.rerender(
      <Provider store={store}>
        <Sync edges={[one, { ...two, portStats: [{ key: 'rows', value: 2 }] }]} />
      </Provider>
    );
    expect(store.get(inspectedPipeAtom)?.portStats).toEqual([{ key: 'rows', value: 2 }]);
    act(() => {
      store.set(operatorSelectionActionAtom, { type: 'hydrate', selections: [] });
    });
    expect(store.get(inspectedPipeAtom)?.id).toBe('two');
    act(() => {
      store.set(operatorSelectionActionAtom, { type: 'replace', selections: [] });
    });
    expect(store.get(inspectedPipeRefAtom)).toBeNull();
  });
  it('clears an inspection whose exact endpoints disappear', () => {
    const store = createStore();
    const view = render(
      <Provider store={store}>
        <Sync edges={[one, two]} />
      </Provider>
    );
    act(() => {
      store.set(inspectedPipeRefAtom, { sourcePortId: 'out-2', targetPortId: 'in-2' });
    });
    view.rerender(
      <Provider store={store}>
        <Sync edges={[one]} />
      </Provider>
    );
    expect(store.get(inspectedPipeRefAtom)).toBeNull();
  });
});
