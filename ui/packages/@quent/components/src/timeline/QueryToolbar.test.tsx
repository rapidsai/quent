// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect } from 'react';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Provider } from 'jotai';
import { describe, expect, it } from 'vitest';
import {
  useSelectedNodeData,
  useSelectedNodeIds,
  useSelectedOperatorLabel,
  useSetSelectedNodeData,
  useSetSelectedNodeIds,
  useSetSelectedOperatorLabel,
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

function OperatorLabel() {
  return <span data-testid="operator-label">{useSelectedOperatorLabel() ?? 'none'}</span>;
}

function OperatorSelectionProbe() {
  const selectedNodeIds = useSelectedNodeIds();
  const selectedNodeData = useSelectedNodeData();
  return (
    <>
      <span data-testid="selected-count">{selectedNodeIds.size}</span>
      <span data-testid="selected-details">{selectedNodeData?.nodeId ?? 'none'}</span>
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

    await user.click(await screen.findByRole('button', { name: 'Clear operator filter Scan' }));
    expect(screen.getByTestId('operator-label')).toHaveTextContent('none');
    expect(screen.getByRole('textbox', { name: 'Filter resources' })).toHaveValue('id:gpu-0');
  });

  it('clears the full operator selection and pinned details', async () => {
    const user = userEvent.setup();
    render(
      <Provider>
        <SeedOperatorFilter />
        <OperatorSelectionProbe />
        <QueryToolbar />
      </Provider>
    );

    await user.click(await screen.findByRole('button', { name: 'Clear operator filter Scan' }));

    expect(screen.getByTestId('selected-count')).toHaveTextContent('0');
    expect(screen.getByTestId('selected-details')).toHaveTextContent('none');
  });
});
