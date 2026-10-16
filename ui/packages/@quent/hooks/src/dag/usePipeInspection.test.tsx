// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createStore, Provider } from 'jotai';
import { act, render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import {
  hoveredPipeIdAtom,
  addInspectedPipeAtom,
  clearInspectedPipesAtom,
  inspectedPipeNodesAtom,
  inspectedPipesAtom,
  inspectedPipeRefsAtom,
  removeInspectedPipeAtom,
  toggleInspectedPipeAtom,
} from '../atoms/pipeInspection';
import { operatorSelectionActionAtom, selectedOperatorIdsAtom } from '../atoms/dag';
import { useSyncDisplayedPipes } from './usePipeInspection';
import type { DAGEdge, DAGNode } from '@quent/utils';

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
  it('drops a ref that never resolved against any plan, such as a stale link', () => {
    const store = createStore();
    const view = render(
      <Provider store={store}>
        <Sync edges={[one]} />
      </Provider>
    );
    act(() => {
      store.set(inspectedPipeRefsAtom, [
        { sourcePortId: 'out-1', targetPortId: 'in-1' },
        { sourcePortId: 'gone', targetPortId: 'gone' },
      ]);
    });
    view.rerender(
      <Provider store={store}>
        <Sync edges={[one, two]} />
      </Provider>
    );
    expect(store.get(inspectedPipeRefsAtom)).toEqual([
      { sourcePortId: 'out-1', targetPortId: 'in-1' },
    ]);
  });
});

describe('pipes across plans', () => {
  const otherPlan: DAGEdge = {
    id: 'other',
    source: 'x',
    target: 'y',
    sourcePortId: 'out-x',
    targetPortId: 'in-y',
  };
  const nodes = [
    { id: 's', label: 'Scan', type: 'scan' },
    { id: 't', label: 'Join', type: 'join' },
  ];
  function SyncWithNodes({ edges, nodes }: { edges: DAGEdge[]; nodes: DAGNode[] }) {
    useSyncDisplayedPipes(edges, nodes);
    return null;
  }

  it('keeps pipes from a previous plan inspected, alongside new ones', () => {
    const store = createStore();
    const view = render(
      <Provider store={store}>
        <SyncWithNodes edges={[one]} nodes={nodes} />
      </Provider>
    );
    act(() => {
      store.set(addInspectedPipeAtom, { sourcePortId: 'out-1', targetPortId: 'in-1' });
    });
    view.rerender(
      <Provider store={store}>
        <SyncWithNodes edges={[otherPlan]} nodes={[]} />
      </Provider>
    );
    act(() => {
      store.set(addInspectedPipeAtom, { sourcePortId: 'out-x', targetPortId: 'in-y' });
    });
    expect(store.get(inspectedPipesAtom).map(pipe => pipe.id)).toEqual(['one', 'other']);
    // Endpoint operators of the earlier plan's pipe are still resolvable for display.
    expect(store.get(inspectedPipeNodesAtom).get('s')?.label).toBe('Scan');
  });

  it('removes a pipe from another plan once it is deselected', () => {
    const store = createStore();
    const view = render(
      <Provider store={store}>
        <SyncWithNodes edges={[one]} nodes={nodes} />
      </Provider>
    );
    act(() => {
      store.set(addInspectedPipeAtom, { sourcePortId: 'out-1', targetPortId: 'in-1' });
    });
    view.rerender(
      <Provider store={store}>
        <SyncWithNodes edges={[otherPlan]} nodes={[]} />
      </Provider>
    );
    act(() => {
      store.set(removeInspectedPipeAtom, { sourcePortId: 'out-1', targetPortId: 'in-1' });
    });
    expect(store.get(inspectedPipesAtom)).toEqual([]);
    expect(store.get(inspectedPipeNodesAtom).size).toBe(0);
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

  it('removing drops only the matching pipe and ignores one that is not inspected', () => {
    const store = createStore();
    store.set(addInspectedPipeAtom, first);
    store.set(addInspectedPipeAtom, second);
    store.set(removeInspectedPipeAtom, { ...first });
    expect(store.get(inspectedPipeRefsAtom)).toEqual([second]);
    store.set(removeInspectedPipeAtom, { ...first });
    expect(store.get(inspectedPipeRefsAtom)).toEqual([second]);
  });

  it('clearing drops every pipe and leaves the operator selection alone', () => {
    const store = createStore();
    store.set(operatorSelectionActionAtom, {
      type: 'replace',
      selections: [{ selectionId: 's', label: 'Scan', operatorIds: new Set(['s']) }],
    });
    store.set(addInspectedPipeAtom, first);
    store.set(addInspectedPipeAtom, second);
    store.set(clearInspectedPipesAtom);
    expect(store.get(inspectedPipeRefsAtom)).toEqual([]);
    expect(store.get(selectedOperatorIdsAtom)).toEqual(new Set(['s']));
  });

  it('does not treat pipes that share only one port as the same pipe', () => {
    const store = createStore();
    store.set(toggleInspectedPipeAtom, first);
    store.set(toggleInspectedPipeAtom, { ...first, targetPortId: 'in-2' });
    expect(store.get(inspectedPipeRefsAtom)).toEqual([first, { ...first, targetPortId: 'in-2' }]);
  });
});
