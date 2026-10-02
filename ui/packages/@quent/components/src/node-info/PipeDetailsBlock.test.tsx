// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { fireEvent, render, screen } from '@testing-library/react';
import { createStore, Provider } from 'jotai';
import { expect, it } from 'vitest';
import {
  displayedDagNodesAtom,
  inspectedPipeRefAtom,
} from '../../../hooks/src/atoms/pipeInspection';
import { operatorSelectionActionAtom, selectedOperatorIdsAtom } from '../../../hooks/src/atoms/dag';
import {
  highlightedNodeIdsAtom,
  selectedOperatorsDataAtom,
} from '../../../hooks/src/atoms/dagControls';
import { PipeDetailsBlock } from './PipeDetailsBlock';

it('presents operator labels and highlights the matching endpoint on hover and focus', () => {
  const store = createStore();
  store.set(displayedDagNodesAtom, [
    { id: 'scan', label: 'Parquet scan [6]', type: 'scan' },
    { id: 'join', label: 'Partitioned join [42]', type: 'join' },
  ]);
  const { unmount } = render(
    <Provider store={store}>
      <PipeDetailsBlock
        pipe={{
          id: 'pipe',
          source: 'scan',
          target: 'join',
          sourcePortId: 'out',
          targetPortId: 'in',
          sourcePortName: 'shuffled',
          targetPortName: 'build',
        }}
      />
    </Provider>
  );
  expect(screen.getByText('Parquet scan [6]')).toBeVisible();
  expect(screen.getByText('Partitioned join [42]')).toBeVisible();
  fireEvent.mouseEnter(screen.getByRole('region', { name: 'Sending port' }));
  expect(store.get(highlightedNodeIdsAtom).ids).toEqual(new Set(['scan']));
  fireEvent.mouseLeave(screen.getByRole('region', { name: 'Sending port' }));
  expect(store.get(highlightedNodeIdsAtom).ids).toBeNull();
  fireEvent.focus(screen.getByText('Partitioned join [42]'));
  expect(store.get(highlightedNodeIdsAtom).primaryOperatorId).toBe('join');
  fireEvent.blur(screen.getByText('Partitioned join [42]'));
  expect(store.get(highlightedNodeIdsAtom).ids).toBeNull();
  fireEvent.mouseEnter(screen.getByRole('region', { name: 'Receiving port' }));
  unmount();
  expect(store.get(highlightedNodeIdsAtom).ids).toBeNull();
});

it('navigates from either endpoint to its operator, replacing filters and leaving pipe inspection', () => {
  const store = createStore();
  store.set(displayedDagNodesAtom, [
    {
      id: 'scan',
      label: 'Parquet scan [6]',
      type: 'scan',
      metadata: { relatedOperatorIds: ['scan-worker'] },
    },
    { id: 'join', label: 'Partitioned join [42]', type: 'join' },
  ]);
  store.set(operatorSelectionActionAtom, {
    type: 'replace',
    selections: [{ selectionId: 'other', label: 'Other', operatorIds: ['other'] }],
  });
  store.set(inspectedPipeRefAtom, { sourcePortId: 'out', targetPortId: 'in' });
  render(
    <Provider store={store}>
      <PipeDetailsBlock
        pipe={{
          id: 'pipe',
          source: 'scan',
          target: 'join',
          sourcePortId: 'out',
          targetPortId: 'in',
        }}
      />
    </Provider>
  );
  const sending = screen.getByRole('region', { name: 'Sending port' });
  fireEvent.mouseEnter(sending);
  fireEvent.click(sending);
  expect(store.get(inspectedPipeRefAtom)).toBeNull();
  expect(store.get(selectedOperatorIdsAtom)).toEqual(new Set(['scan', 'scan-worker']));
  expect(store.get(selectedOperatorsDataAtom).get('scan')?.label).toBe('Parquet scan [6]');
  fireEvent.click(
    screen.getByRole('button', { name: 'Show Partitioned join [42] operator details' })
  );
  expect(store.get(selectedOperatorIdsAtom)).toEqual(new Set(['join']));
  expect(store.get(selectedOperatorsDataAtom).get('join')?.label).toBe('Partitioned join [42]');
});
