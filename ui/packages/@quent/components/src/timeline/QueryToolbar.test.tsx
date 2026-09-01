// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect } from 'react';
import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Provider } from 'jotai';
import { describe, expect, it } from 'vitest';
import {
  useSelectedNodeData,
  useSelectedNodeIds,
  useSelectedNodesData,
  useSelectedOperatorLabel,
  useSetSelectedNodeData,
  useSetSelectedNodeIds,
  useSetSelectedOperatorLabel,
  useOperatorSelectionActions,
} from '@quent/hooks';
import { QueryToolbar } from './QueryToolbar';

function SeedOperatorFilter() {
  const setNodeIds = useSetSelectedNodeIds();
  const setLabel = useSetSelectedOperatorLabel();
  const setNodeData = useSetSelectedNodeData();

  useEffect(() => {
    setNodeIds(new Set(['operator-1']));
    setLabel('Scan');
    setNodeData({
      nodeId: 'operator-1',
      label: 'Scan',
      operationType: 'logical',
      statistics: [],
    });
  }, [setLabel, setNodeData, setNodeIds]);
  return null;
}

function ToolbarHarness() {
  const selectedNodeIds = useSelectedNodeIds();
  const selectedNodeData = useSelectedNodeData();
  const updateOperatorSelection = useOperatorSelectionActions();

  useEffect(() => {
    updateOperatorSelection({
      type: 'add',
      selectionId: 'parent',
      label: 'Parent operator',
      operatorIds: ['parent', 'child'],
      inspectedData: {
        nodeId: 'parent',
        label: 'Parent operator',
        operationType: 'logical',
        statistics: [],
      },
    });
  }, [updateOperatorSelection]);

  return (
    <>
      <QueryToolbar />
      <span data-testid="selected-count">{selectedNodeIds.size}</span>
      <span data-testid="selected-details">{selectedNodeData?.nodeId ?? 'none'}</span>
    </>
  );
}

function OperatorLabel() {
  return <span data-testid="operator-label">{useSelectedOperatorLabel() ?? 'none'}</span>;
}

function MultiOperatorToolbarHarness() {
  const selectedNodeIds = useSelectedNodeIds();
  const selectedNodes = useSelectedNodesData();
  const updateOperatorSelection = useOperatorSelectionActions();

  useEffect(() => {
    for (let index = 0; index < 5; index += 1) {
      const number = index + 1;
      const id = `operator-${number}`;
      updateOperatorSelection({
        type: 'add',
        selectionId: id,
        label: `Operator ${number}`,
        operatorIds: [id],
        inspectedData: {
          nodeId: id,
          label: `Operator ${number}`,
          operationType: 'physical',
          statistics: [],
        },
      });
    }
  }, [updateOperatorSelection]);

  return (
    <>
      <QueryToolbar />
      <span data-testid="selected-count">{selectedNodeIds.size}</span>
      <span data-testid="inspected-count">{selectedNodes.length}</span>
    </>
  );
}

function TwoOperatorToolbarHarness() {
  const selectedNodes = useSelectedNodesData();
  const updateOperatorSelection = useOperatorSelectionActions();

  useEffect(() => {
    updateOperatorSelection({
      type: 'add',
      selectionId: 'scan',
      label: 'Scan',
      operatorIds: ['scan'],
      inspectedData: {
        nodeId: 'scan',
        label: 'Scan',
        operationType: 'scan',
        statistics: [],
      },
    });
    updateOperatorSelection({
      type: 'add',
      selectionId: 'join',
      label: 'Join',
      operatorIds: ['join'],
      inspectedData: {
        nodeId: 'join',
        label: 'Join',
        operationType: 'join',
        statistics: [],
      },
    });
  }, [updateOperatorSelection]);

  return (
    <>
      <QueryToolbar />
      <span data-testid="inspected-ids">{selectedNodes.map(node => node.nodeId).join(',')}</span>
    </>
  );
}

describe('QueryToolbar', () => {
  it('shows custom resource filters before the active operator filter', async () => {
    render(
      <Provider>
        <SeedOperatorFilter />
        <QueryToolbar filters={<input aria-label="Filter resources" />} />
      </Provider>
    );

    const operatorFilter = await screen.findByText('Scan');
    const resourceFilters = screen.getByRole('textbox', { name: 'Filter resources' });
    expect(resourceFilters.compareDocumentPosition(operatorFilter)).toBe(
      Node.DOCUMENT_POSITION_FOLLOWING
    );
    expect(screen.queryByText('No filters')).not.toBeInTheDocument();
  });

  it('clears only the operator filter', async () => {
    const user = userEvent.setup();
    render(
      <Provider>
        <SeedOperatorFilter />
        <OperatorLabel />
        <QueryToolbar filters={<input aria-label="Filter resources" value="id:gpu-0" readOnly />} />
      </Provider>
    );

    await user.click(await screen.findByRole('button', { name: 'Clear all operator filters' }));
    expect(screen.getByTestId('operator-label')).toHaveTextContent('none');
    expect(screen.getByRole('textbox', { name: 'Filter resources' })).toHaveValue('id:gpu-0');
  });

  it('clears the full operator selection and pinned details', async () => {
    const user = userEvent.setup();
    render(
      <Provider>
        <ToolbarHarness />
      </Provider>
    );

    await user.click(await screen.findByRole('button', { name: 'Clear all operator filters' }));

    expect(screen.getByTestId('selected-count')).toHaveTextContent('0');
    expect(screen.getByTestId('selected-details')).toHaveTextContent('none');
  });

  it('caps badges and supports individual and bulk clearing', async () => {
    render(
      <Provider>
        <MultiOperatorToolbarHarness />
      </Provider>
    );

    expect(await screen.findByText('Operator 1')).toBeInTheDocument();
    expect(screen.getByText('Operator 2')).toBeInTheDocument();
    expect(screen.getByText('Operator 3')).toBeInTheDocument();
    expect(screen.queryByText('Operator 4')).not.toBeInTheDocument();
    expect(screen.getByText('and 2 more')).toBeInTheDocument();
    expect(screen.getByText('and 2 more')).toHaveAttribute('title', 'Operator 4, Operator 5');
    expect(screen.getByTestId('operator-filter-badges')).toHaveClass('max-w-[40%]');

    fireEvent.click(screen.getByRole('button', { name: 'Remove Operator 2' }));

    expect(screen.getByTestId('selected-count')).toHaveTextContent('4');
    expect(screen.getByTestId('inspected-count')).toHaveTextContent('4');
    expect(screen.getByText('Operator 4')).toBeInTheDocument();
    expect(screen.getByText('and 1 more')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Clear all operator filters' }));

    expect(screen.getByTestId('selected-count')).toHaveTextContent('0');
    expect(screen.getByTestId('inspected-count')).toHaveTextContent('0');
    expect(screen.getByText('No filters')).toBeInTheDocument();
  });

  it('keeps remaining operator details after removing the last-clicked badge', async () => {
    render(
      <Provider>
        <TwoOperatorToolbarHarness />
      </Provider>
    );

    fireEvent.click(await screen.findByRole('button', { name: 'Remove Join' }));

    expect(screen.getByTestId('inspected-ids')).toHaveTextContent('scan');
    expect(screen.getByText('Scan')).toBeInTheDocument();
    expect(screen.queryByText('Join')).not.toBeInTheDocument();
  });
});
