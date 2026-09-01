// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useState } from 'react';
import { fireEvent, render, screen, within } from '@testing-library/react';
import { Provider } from 'jotai';
import { useOperatorSelectionActions, useSetSelectedNodeData } from '@quent/hooks';
import { getOperationTypeColor } from '@quent/utils';
import { DAGNodeInfoPanel } from './DAGNodeInfoPanel';

function SelectedNode() {
  const setSelectedNodeData = useSetSelectedNodeData();

  useEffect(() => {
    setSelectedNodeData({
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
    });
  }, [setSelectedNodeData]);

  return <DAGNodeInfoPanel />;
}

function SwitchSelectedNode() {
  const [showLogical, setShowLogical] = useState(true);
  const setSelectedNodeData = useSetSelectedNodeData();

  useEffect(() => {
    setSelectedNodeData(
      showLogical
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
          }
    );
  }, [setSelectedNodeData, showLogical]);

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
      inspectedData: {
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
      inspectedData: {
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
  it('shows statistics for every related child operator', async () => {
    render(
      <Provider>
        <SelectedNode />
      </Provider>
    );

    const title = await screen.findByTestId('operator-details-title');
    expect(within(title).getByText('Logical join')).toHaveAttribute('title', 'Logical join');
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
      backgroundColor: getOperationTypeColor('hashbuild'),
    });
    expect(bars.find(bar => bar.getAttribute('data-operation-type') === 'hashprobe')).toHaveStyle({
      backgroundColor: getOperationTypeColor('hashprobe'),
    });
  });

  it('shows statistics for every selected operator', async () => {
    render(
      <Provider>
        <TwoSelectedNodes />
      </Provider>
    );

    const title = await screen.findByTestId('operator-details-title');
    expect(within(title).getByText('Table scan')).toHaveAttribute('title', 'Table scan');
    expect(within(title).getByText('Hash join')).toHaveAttribute('title', 'Hash join');
    expect(screen.getByText('output rows:')).toBeInTheDocument();
    expect(screen.getByText('build rows:')).toBeInTheDocument();
    expect(screen.getByText('Probe hash table')).toBeInTheDocument();
    expect(screen.getByText('probe rows:')).toBeInTheDocument();

    const titleBars = within(title).getAllByTestId('operator-color-bar');
    expect(titleBars[0]).toHaveAttribute('data-operation-type', 'scan');
    expect(titleBars[0]).toHaveStyle({ backgroundColor: getOperationTypeColor('scan') });
    expect(titleBars[1]).toHaveAttribute('data-operation-type', 'hashjoin');
    expect(titleBars[1]).toHaveStyle({ backgroundColor: getOperationTypeColor('hashjoin') });
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
