// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { act, fireEvent, render, screen, within } from '@testing-library/react';
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
  // Each operator shows as a pill in the entry header and again inside its port section.
  const header = screen.getByRole('button', {
    name: 'Toggle pipe from Parquet scan [6] to Partitioned join [42] details',
  });
  expect(within(header).getByText('Parquet scan [6]')).toBeVisible();
  expect(within(header).getByText('Partitioned join [42]')).toBeVisible();
  expect(screen.getAllByText('Parquet scan [6]')).toHaveLength(2);
  expect(screen.getAllByText('Partitioned join [42]')).toHaveLength(2);
  fireEvent.mouseEnter(screen.getByRole('region', { name: 'Sending port' }));
  expect(probe.latest().highlight.ids).toEqual(new Set(['scan']));
  fireEvent.mouseLeave(screen.getByRole('region', { name: 'Sending port' }));
  expect(probe.latest().highlight.ids).toBeNull();
  const receiving = screen.getByRole('region', { name: 'Receiving port' });
  fireEvent.focus(within(receiving).getByText('Partitioned join [42]'));
  expect(probe.latest().highlight.primaryOperatorId).toBe('join');
  fireEvent.blur(within(receiving).getByText('Partitioned join [42]'));
  expect(probe.latest().highlight.ids).toBeNull();
  fireEvent.mouseEnter(receiving);
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

it('collapses and expands its ports from the header', () => {
  const probe = createPipeInspectionRecorder();
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
  };
  render(
    <Provider store={createStore()}>
      <PipeInspectionProbe nodes={nodes} edges={[pipe]} recorder={probe}>
        <PipeDetailsBlock pipe={pipe} />
      </PipeInspectionProbe>
    </Provider>
  );
  const toggle = screen.getByRole('button', {
    name: 'Toggle pipe from Parquet scan [6] to Partitioned join [42] details',
  });
  expect(screen.getByRole('region', { name: 'Sending port' })).toBeVisible();
  fireEvent.click(toggle);
  expect(screen.queryByRole('region', { name: 'Sending port' })).not.toBeInTheDocument();
  fireEvent.click(toggle);
  expect(screen.getByRole('region', { name: 'Sending port' })).toBeVisible();
});
