// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect } from 'react';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ReactFlow, ReactFlowProvider } from '@xyflow/react';
import { Provider } from 'jotai';
import { describe, expect, it } from 'vitest';
import {
  useAddInspectedPipe,
  useInspectedPipes,
  useOperatorSelectionActions,
  useSelectedOperatorIds,
  useSyncDisplayedPipes,
} from '@quent/hooks';
import { DAGClearSelection } from './DAGClearSelection';

const EDGES = [{ id: 'e1', source: 'a', target: 'b', sourcePortId: 'out', targetPortId: 'in' }];

function Harness() {
  const operatorIds = useSelectedOperatorIds();
  const pipes = useInspectedPipes();
  const addPipe = useAddInspectedPipe();
  const updateOperatorSelection = useOperatorSelectionActions();
  useSyncDisplayedPipes(EDGES);

  useEffect(() => {
    updateOperatorSelection({
      type: 'add',
      selectionId: 'scan',
      label: 'Scan',
      operatorIds: ['scan'],
      selectedData: { nodeId: 'scan', label: 'Scan', operationType: 'scan', statistics: [] },
    });
  }, [updateOperatorSelection]);

  return (
    <>
      <div style={{ width: 400, height: 300 }}>
        <ReactFlow nodes={[]} edges={[]}>
          <DAGClearSelection />
        </ReactFlow>
      </div>
      <button type="button" onClick={() => addPipe({ sourcePortId: 'out', targetPortId: 'in' })}>
        Add pipe
      </button>
      <span data-testid="operators">{operatorIds.size}</span>
      <span data-testid="pipes">{pipes.length}</span>
    </>
  );
}

describe('DAGClearSelection', () => {
  it('clears operators and pipes together, then hides itself', async () => {
    const user = userEvent.setup();
    render(
      <Provider>
        <ReactFlowProvider>
          <Harness />
        </ReactFlowProvider>
      </Provider>
    );

    await user.click(await screen.findByRole('button', { name: 'Add pipe' }));
    expect(screen.getByTestId('operators')).toHaveTextContent('1');
    expect(screen.getByTestId('pipes')).toHaveTextContent('1');

    await user.click(screen.getByRole('button', { name: 'Clear selection' }));

    expect(screen.getByTestId('operators')).toHaveTextContent('0');
    expect(screen.getByTestId('pipes')).toHaveTextContent('0');
    expect(screen.queryByRole('button', { name: 'Clear selection' })).not.toBeInTheDocument();
  });

  it('renders nothing when nothing is selected', () => {
    render(
      <Provider>
        <ReactFlow nodes={[]} edges={[]}>
          <DAGClearSelection />
        </ReactFlow>
      </Provider>
    );
    expect(screen.queryByRole('button', { name: 'Clear selection' })).not.toBeInTheDocument();
  });
});
