// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createStore, Provider } from 'jotai';
import { act, render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import {
  hoveredPipeIdAtom,
  addInspectedPipeAtom,
  inspectedPipesAtom,
  inspectedPipeRefsAtom,
  toggleInspectedPipeAtom,
} from '../atoms/pipeInspection';
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
      store.set(inspectedPipeRefsAtom, [{ sourcePortId: 'out-2', targetPortId: 'in-2' }]);
    });
    expect(store.get(inspectedPipesAtom).map(pipe => pipe.id)).toEqual(['two']);
    expect(store.get(selectedOperatorIdsAtom)).toEqual(new Set(['s']));
    view.rerender(
      <Provider store={store}>
        <Sync edges={[one, { ...two, portStats: [{ key: 'rows', value: 2 }] }]} />
      </Provider>
    );
    expect(store.get(inspectedPipesAtom)[0]?.portStats).toEqual([{ key: 'rows', value: 2 }]);
    act(() => {
      store.set(operatorSelectionActionAtom, { type: 'hydrate', selections: [] });
    });
    expect(store.get(inspectedPipesAtom).map(pipe => pipe.id)).toEqual(['two']);
    // Operator selection changes never touch inspected pipes.
    act(() => {
      store.set(operatorSelectionActionAtom, { type: 'replace', selections: [] });
    });
    expect(store.get(inspectedPipesAtom).map(pipe => pipe.id)).toEqual(['two']);
  });
  it('clears an inspection whose exact endpoints disappear', () => {
    const store = createStore();
    const view = render(
      <Provider store={store}>
        <Sync edges={[one, two]} />
      </Provider>
    );
    act(() => {
      store.set(inspectedPipeRefsAtom, [
        { sourcePortId: 'out-1', targetPortId: 'in-1' },
        { sourcePortId: 'out-2', targetPortId: 'in-2' },
      ]);
    });
    view.rerender(
      <Provider store={store}>
        <Sync edges={[one]} />
      </Provider>
    );
    expect(store.get(inspectedPipeRefsAtom)).toEqual([
      { sourcePortId: 'out-1', targetPortId: 'in-1' },
    ]);
  });
});

it('clears a transient edge highlight on plan changes and unmount', () => {
  const store = createStore();
  const view = render(
    <Provider store={store}>
      <Sync edges={[one]} />
    </Provider>
  );
  act(() => store.set(hoveredPipeIdAtom, one.id));
  view.rerender(
    <Provider store={store}>
      <Sync edges={[two]} />
    </Provider>
  );
  expect(store.get(hoveredPipeIdAtom)).toBeNull();
  act(() => store.set(hoveredPipeIdAtom, two.id));
  view.unmount();
  expect(store.get(hoveredPipeIdAtom)).toBeNull();
});

describe('inspected pipe actions', () => {
  const first = { sourcePortId: 'out-1', targetPortId: 'in-1' };
  const second = { sourcePortId: 'out-2', targetPortId: 'in-2' };

  it('toggling adds pipes in selection order and removes only the toggled pipe', () => {
    const store = createStore();
    store.set(toggleInspectedPipeAtom, first);
    store.set(toggleInspectedPipeAtom, second);
    expect(store.get(inspectedPipeRefsAtom)).toEqual([first, second]);
    store.set(toggleInspectedPipeAtom, { ...first });
    expect(store.get(inspectedPipeRefsAtom)).toEqual([second]);
  });

  it('adding never removes, and ignores a pipe that is already inspected', () => {
    const store = createStore();
    store.set(addInspectedPipeAtom, first);
    store.set(addInspectedPipeAtom, { ...first });
    store.set(addInspectedPipeAtom, second);
    expect(store.get(inspectedPipeRefsAtom)).toEqual([first, second]);
  });

  it('does not treat pipes that share only one port as the same pipe', () => {
    const store = createStore();
    store.set(toggleInspectedPipeAtom, first);
    store.set(toggleInspectedPipeAtom, { ...first, targetPortId: 'in-2' });
    expect(store.get(inspectedPipeRefsAtom)).toEqual([first, { ...first, targetPortId: 'in-2' }]);
  });
});
