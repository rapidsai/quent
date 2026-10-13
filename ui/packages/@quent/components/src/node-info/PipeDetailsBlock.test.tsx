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

it('navigates from either endpoint to its operator, replacing filters and leaving pipe inspection', () => {
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
    probe.latest().setInspectedPipe({ sourcePortId: 'out', targetPortId: 'in' });
  });
  expect(probe.latest().inspectedPipe).not.toBeNull();
  const sending = screen.getByRole('region', { name: 'Sending port' });
  fireEvent.mouseEnter(sending);
  fireEvent.click(sending);
  expect(probe.latest().inspectedPipe).toBeNull();
  expect(probe.latest().selectedOperatorIds).toEqual(new Set(['scan', 'scan-worker']));
  expect(probe.latest().selectedOperatorsData.find(data => data.nodeId === 'scan')?.label).toBe(
    'Parquet scan [6]'
  );
  fireEvent.click(
    screen.getByRole('button', { name: 'Show Partitioned join [42] operator details' })
  );
  expect(probe.latest().selectedOperatorIds).toEqual(new Set(['join']));
  expect(probe.latest().selectedOperatorsData.find(data => data.nodeId === 'join')?.label).toBe(
    'Partitioned join [42]'
  );
});
