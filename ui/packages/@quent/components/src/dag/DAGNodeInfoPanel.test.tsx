// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createStore } from 'jotai';
import { displayedPipesAtom, inspectedPipeRefAtom } from '../../../hooks/src/atoms/pipeInspection';
import { useEffect, useState } from 'react';
import { fireEvent, render, screen, within } from '@testing-library/react';
import { Provider } from 'jotai';
import { useOperatorSelectionActions } from '@quent/hooks';
import { getDeterministicColor } from '@quent/utils';
import { DAGNodeInfoPanel } from './DAGNodeInfoPanel';

function SelectedNode({ onExpandedChange }: { onExpandedChange?: (expanded: boolean) => void }) {
  const updateOperatorSelection = useOperatorSelectionActions();

  useEffect(() => {
    updateOperatorSelection({
      type: 'add',
      selectionId: 'logical',
      label: 'Logical join',
      operatorIds: ['logical', 'physical-1', 'physical-2'],
      selectedData: {
        nodeId: 'logical',
        label: 'Logical join',
        operationType: 'logicaljoin',
        statistics: [{ key: 'logical_rows', value: 10 }],
        relatedOperators: [
          {
            nodeId: 'physical-1',
            label: 'Build hash table',
            operationType: 'hashbuild',
            statistics: [{ key: 'build_rows', value: 20 }],
          },
          {
            nodeId: 'physical-2',
            label: 'Probe hash table',
            operationType: 'hashprobe',
            statistics: [{ key: 'probe_rows', value: 30 }],
          },
        ],
      },
    });
  }, [updateOperatorSelection]);

  return <DAGNodeInfoPanel onExpandedChange={onExpandedChange} />;
}

function SameNameOnDifferentWorkers() {
  const updateOperatorSelection = useOperatorSelectionActions();

  useEffect(() => {
    updateOperatorSelection({
      type: 'add',
      selectionId: 'scan',
      label: 'Table scan',
      operatorIds: ['scan-1', 'scan-2'],
      selectedData: {
        nodeId: 'scan-1',
        label: 'Table scan',
        operationType: 'scan',
        statistics: [{ key: 'output_rows', value: 10 }],
        workerLabel: 'worker-1',
        relatedOperators: [
          {
            nodeId: 'scan-2',
            label: 'Table scan',
            operationType: 'scan',
            statistics: [{ key: 'output_rows', value: 20 }],
            workerLabel: 'worker-2',
          },
        ],
      },
    });
  }, [updateOperatorSelection]);

  return <DAGNodeInfoPanel />;
}

function SwitchSelectedNode() {
  const [showLogical, setShowLogical] = useState(true);
  const updateOperatorSelection = useOperatorSelectionActions();

  useEffect(() => {
    const data = showLogical
      ? {
          nodeId: 'logical',
          label: 'Logical join',
          operationType: 'logicaljoin',
          statistics: [],
        }
      : {
          nodeId: 'scan',
          label: 'Table scan',
          operationType: 'scan',
          statistics: [],
        };
    updateOperatorSelection({
      type: 'replace',
      selections: [
        {
          selectionId: data.nodeId,
          label: data.label,
          operatorIds: new Set([data.nodeId]),
          selectedData: data,
        },
      ],
    });
  }, [showLogical, updateOperatorSelection]);

  return (
    <>
      <button onClick={() => setShowLogical(value => !value)}>Switch operator</button>
      <DAGNodeInfoPanel />
    </>
  );
}

function TwoSelectedNodes() {
  const updateOperatorSelection = useOperatorSelectionActions();

  useEffect(() => {
    updateOperatorSelection({
      type: 'add',
      selectionId: 'scan',
      label: 'Table scan',
      operatorIds: ['scan'],
      selectedData: {
        nodeId: 'scan',
        label: 'Table scan',
        operationType: 'scan',
        statistics: [{ key: 'output_rows', value: 10 }],
      },
    });
    updateOperatorSelection({
      type: 'add',
      selectionId: 'join',
      label: 'Hash join',
      operatorIds: ['join'],
      selectedData: {
        nodeId: 'join',
        label: 'Hash join',
        operationType: 'hashjoin',
        statistics: [{ key: 'build_rows', value: 20 }],
        relatedOperators: [
          {
            nodeId: 'probe',
            label: 'Probe hash table',
            operationType: 'hashprobe',
            statistics: [{ key: 'probe_rows', value: 30 }],
          },
        ],
      },
    });
  }, [updateOperatorSelection]);

  return <DAGNodeInfoPanel />;
}

describe('DAGNodeInfoPanel', () => {
  it('reports expansion changes to its layout container', async () => {
    const onExpandedChange = vi.fn();
    render(
      <Provider>
        <SelectedNode onExpandedChange={onExpandedChange} />
      </Provider>
    );

    await screen.findByTestId('operator-accordion-logical');
    expect(onExpandedChange).toHaveBeenLastCalledWith(true);

    fireEvent.click(screen.getByRole('button', { name: 'Toggle operator details' }));

    expect(onExpandedChange).toHaveBeenLastCalledWith(false);
  });

  it('shows statistics for every related child operator', async () => {
    render(
      <Provider>
        <SelectedNode />
      </Provider>
    );

    const accordion = await screen.findByTestId('operator-accordion-logical');
    expect(within(accordion).getByText('Logical join')).toHaveAttribute('title', 'Logical join');
    expect(screen.getByText('Build hash table')).toBeInTheDocument();
    expect(screen.getByText('Probe hash table')).toBeInTheDocument();
    expect(screen.getByText('build rows:')).toBeInTheDocument();
    expect(screen.getByText('probe rows:')).toBeInTheDocument();
    expect(screen.getByText('physical-1')).toBeInTheDocument();
    expect(screen.getByText('physical-2')).toBeInTheDocument();

    const bars = screen.getAllByTestId('operator-color-bar');
    expect(
      bars.filter(bar => bar.getAttribute('data-operation-type') === 'logicaljoin')
    ).not.toHaveLength(0);
    expect(bars.find(bar => bar.getAttribute('data-operation-type') === 'hashbuild')).toHaveStyle({
      backgroundColor: getDeterministicColor('hashbuild'),
    });
    expect(bars.find(bar => bar.getAttribute('data-operation-type') === 'hashprobe')).toHaveStyle({
      backgroundColor: getDeterministicColor('hashprobe'),
    });
  });

  it('shows statistics for every selected operator', async () => {
    render(
      <Provider>
        <TwoSelectedNodes />
      </Provider>
    );

    const scanAccordion = await screen.findByTestId('operator-accordion-scan');
    const joinAccordion = screen.getByTestId('operator-accordion-join');
    expect(within(scanAccordion).getByText('Table scan')).toHaveAttribute('title', 'Table scan');
    expect(within(joinAccordion).getByText('Hash join')).toHaveAttribute('title', 'Hash join');
    expect(screen.getByText('output rows:')).toBeInTheDocument();
    expect(screen.getByText('build rows:')).toBeInTheDocument();
    expect(screen.getByText('Probe hash table')).toBeInTheDocument();
    expect(screen.getByText('probe rows:')).toBeInTheDocument();

    const scanBar = within(scanAccordion).getAllByTestId('operator-color-bar')[0];
    expect(scanBar).toHaveAttribute('data-operation-type', 'scan');
    expect(scanBar).toHaveStyle({ backgroundColor: getDeterministicColor('scan') });
    const joinBar = within(joinAccordion).getAllByTestId('operator-color-bar')[0];
    expect(joinBar).toHaveAttribute('data-operation-type', 'hashjoin');
    expect(joinBar).toHaveStyle({ backgroundColor: getDeterministicColor('hashjoin') });
  });

  it('collapses a selected operator without hiding the others', async () => {
    render(
      <Provider>
        <TwoSelectedNodes />
      </Provider>
    );

    const scanToggle = await screen.findByRole('button', { name: 'Toggle Table scan details' });
    const joinToggle = screen.getByRole('button', { name: 'Toggle Hash join details' });
    expect(scanToggle).toHaveAttribute('aria-expanded', 'true');
    expect(joinToggle).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByText('output rows:')).toBeInTheDocument();
    expect(screen.getByText('build rows:')).toBeInTheDocument();

    fireEvent.click(scanToggle);

    expect(scanToggle).toHaveAttribute('aria-expanded', 'false');
    expect(joinToggle).toHaveAttribute('aria-expanded', 'true');
    expect(screen.queryByText('output rows:')).not.toBeInTheDocument();
    expect(screen.getByText('build rows:')).toBeInTheDocument();
    expect(screen.getByText('Probe hash table')).toBeInTheDocument();
  });

  it('resets collapsed operators when the selection changes', async () => {
    render(
      <Provider>
        <SwitchSelectedNode />
      </Provider>
    );

    const logicalToggle = await screen.findByRole('button', {
      name: 'Toggle Logical join details',
    });
    fireEvent.click(logicalToggle);
    expect(logicalToggle).toHaveAttribute('aria-expanded', 'false');

    fireEvent.click(screen.getByRole('button', { name: 'Switch operator' }));
    expect(
      await screen.findByRole('button', { name: 'Toggle Table scan details' })
    ).toHaveAttribute('aria-expanded', 'true');

    fireEvent.click(screen.getByRole('button', { name: 'Switch operator' }));
    expect(
      await screen.findByRole('button', { name: 'Toggle Logical join details' })
    ).toHaveAttribute('aria-expanded', 'true');
  });

  it('shows a worker label so same-named operators can be told apart', async () => {
    render(
      <Provider>
        <SameNameOnDifferentWorkers />
      </Provider>
    );

    await screen.findAllByText('Table scan');
    expect(screen.getByText('worker-1')).toBeInTheDocument();
    expect(screen.getByText('worker-2')).toBeInTheDocument();

    // Each toggle must have a distinct accessible name, otherwise screen readers
    // and role-based queries can't tell same-named operators apart.
    expect(
      screen.getByRole('button', { name: 'Toggle Table scan (worker-1) details' })
    ).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: 'Toggle Table scan (worker-2) details' })
    ).toBeInTheDocument();
  });

  it('collapses related child operators independently', async () => {
    render(
      <Provider>
        <SelectedNode />
      </Provider>
    );

    const relatedToggle = await screen.findByRole('button', {
      name: 'Toggle Build hash table details',
    });
    expect(relatedToggle).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByText('build rows:')).toBeInTheDocument();

    fireEvent.click(relatedToggle);

    expect(relatedToggle).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByText('build rows:')).not.toBeInTheDocument();
    expect(screen.getByText('probe rows:')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Toggle Logical join details' })).toHaveAttribute(
      'aria-expanded',
      'true'
    );
  });
});

it('shows nested sending and receiving evidence for an inspected pipe without selecting operators', () => {
  const store = createStore();
  store.set(displayedPipesAtom, [
    {
      id: 'pipe',
      source: 'scan',
      target: 'join',
      sourcePortId: 'out',
      targetPortId: 'in',
      portStats: [
        { key: 'Volume', value: { kind: 'struct', fields: [{ key: 'bytes', value: 1024 }] } },
      ],
      targetPortStats: [
        { key: 'Volume', value: { kind: 'struct', fields: [{ key: 'bytes', value: 512 }] } },
      ],
    },
  ]);
  store.set(inspectedPipeRefAtom, { sourcePortId: 'out', targetPortId: 'in' });
  render(
    <Provider store={store}>
      <DAGNodeInfoPanel />
    </Provider>
  );
  expect(screen.getByText('Pipe Details')).toBeVisible();
  expect(
    within(screen.getByRole('region', { name: 'Sending port' })).getByText('1.00 KiB')
  ).toBeVisible();
  expect(
    within(screen.getByRole('region', { name: 'Receiving port' })).getByText('512.00 B')
  ).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Toggle pipe details' }));
  expect(screen.queryByRole('region', { name: 'Sending port' })).not.toBeInTheDocument();
});
