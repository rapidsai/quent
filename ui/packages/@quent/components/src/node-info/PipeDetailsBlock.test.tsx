// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { act, fireEvent, render, screen } from '@testing-library/react';
import { createStore, Provider } from 'jotai';
import { expect, it } from 'vitest';
import {
  PipeInspectionProbe,
  createPipeInspectionRecorder,
} from '../test-utils/PipeInspectionProbe';
import { PipeDetailsBlock } from './PipeDetailsBlock';

it('presents operator labels and highlights the matching endpoint on hover and focus', () => {
  const probe = createPipeInspectionRecorder();
  const store = createStore();
  const nodes = [
    { id: 'scan', label: 'Parquet scan [6]', type: 'scan' },
    { id: 'join', label: 'Partitioned join [42]', type: 'join' },
  ];
  const pipe = {
    id: 'pipe',
    source: 'scan',
    target: 'join',
    sourcePortId: 'out',
    targetPortId: 'in',
    sourcePortName: 'shuffled',
    targetPortName: 'build',
  };
  const ui = (showBlock: boolean) => (
    <Provider store={store}>
      <PipeInspectionProbe nodes={nodes} edges={[pipe]} recorder={probe}>
        {showBlock && <PipeDetailsBlock pipe={pipe} />}
      </PipeInspectionProbe>
    </Provider>
  );
  const { rerender } = render(ui(true));
  expect(screen.getByText('Parquet scan [6]')).toBeVisible();
  expect(screen.getByText('Partitioned join [42]')).toBeVisible();
  fireEvent.mouseEnter(screen.getByRole('region', { name: 'Sending port' }));
  expect(probe.latest().highlight.ids).toEqual(new Set(['scan']));
  fireEvent.mouseLeave(screen.getByRole('region', { name: 'Sending port' }));
  expect(probe.latest().highlight.ids).toBeNull();
  fireEvent.focus(screen.getByText('Partitioned join [42]'));
  expect(probe.latest().highlight.primaryOperatorId).toBe('join');
  fireEvent.blur(screen.getByText('Partitioned join [42]'));
  expect(probe.latest().highlight.ids).toBeNull();
  fireEvent.mouseEnter(screen.getByRole('region', { name: 'Receiving port' }));
  rerender(ui(false));
  expect(probe.latest().highlight.ids).toBeNull();
});

it('navigates from either endpoint to its operator, adding to the filters and keeping pipe inspection', () => {
  const probe = createPipeInspectionRecorder();
  const store = createStore();
  const nodes = [
    {
      id: 'scan',
      label: 'Parquet scan [6]',
      type: 'scan',
      metadata: { relatedOperatorIds: ['scan-worker'] },
    },
    { id: 'join', label: 'Partitioned join [42]', type: 'join' },
  ];
  const pipe = {
    id: 'pipe',
    source: 'scan',
    target: 'join',
    sourcePortId: 'out',
    targetPortId: 'in',
  };
  render(
    <Provider store={store}>
      <PipeInspectionProbe nodes={nodes} edges={[pipe]} recorder={probe}>
        <PipeDetailsBlock pipe={pipe} />
      </PipeInspectionProbe>
    </Provider>
  );
  act(() => {
    probe.latest().updateSelection({
      type: 'replace',
      selections: [{ selectionId: 'other', label: 'Other', operatorIds: ['other'] }],
    });
    probe.latest().addInspectedPipe({ sourcePortId: 'out', targetPortId: 'in' });
  });
  expect(probe.latest().inspectedPipes).toHaveLength(1);
  const sending = screen.getByRole('region', { name: 'Sending port' });
  fireEvent.mouseEnter(sending);
  fireEvent.click(sending);
  expect(probe.latest().inspectedPipes).toHaveLength(1);
  expect(probe.latest().selectedOperatorIds).toEqual(new Set(['other', 'scan', 'scan-worker']));
  expect(probe.latest().selectedOperatorsData.find(data => data.nodeId === 'scan')?.label).toBe(
    'Parquet scan [6]'
  );
  fireEvent.click(
    screen.getByRole('button', { name: 'Show Partitioned join [42] operator details' })
  );
  expect(probe.latest().selectedOperatorIds).toEqual(
    new Set(['other', 'scan', 'scan-worker', 'join'])
  );
  expect(probe.latest().selectedOperatorsData.find(data => data.nodeId === 'join')?.label).toBe(
    'Partitioned join [42]'
  );
});

it('deselects only its own pipe from the X button, without selecting an endpoint operator', () => {
  const probe = createPipeInspectionRecorder();
  const store = createStore();
  const nodes = [
    { id: 'scan', label: 'Parquet scan [6]', type: 'scan' },
    { id: 'join', label: 'Partitioned join [42]', type: 'join' },
  ];
  const first = {
    id: 'first',
    source: 'scan',
    target: 'join',
    sourcePortId: 'out-1',
    targetPortId: 'in-1',
  };
  const second = { ...first, id: 'second', sourcePortId: 'out-2', targetPortId: 'in-2' };
  render(
    <Provider store={store}>
      <PipeInspectionProbe nodes={nodes} edges={[first, second]} recorder={probe}>
        <PipeDetailsBlock pipe={first} />
      </PipeInspectionProbe>
    </Provider>
  );
  act(() => {
    probe.latest().addInspectedPipe({ sourcePortId: 'out-1', targetPortId: 'in-1' });
    probe.latest().addInspectedPipe({ sourcePortId: 'out-2', targetPortId: 'in-2' });
  });
  expect(probe.latest().inspectedPipes.map(pipe => pipe.id)).toEqual(['first', 'second']);
  fireEvent.click(
    screen.getByRole('button', {
      name: 'Deselect pipe from Parquet scan [6] to Partitioned join [42]',
    })
  );
  expect(probe.latest().inspectedPipes.map(pipe => pipe.id)).toEqual(['second']);
  expect(probe.latest().selectedOperatorIds).toEqual(new Set());
});
