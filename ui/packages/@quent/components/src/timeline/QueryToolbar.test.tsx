// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect } from 'react';
import { fireEvent, render, screen } from '@testing-library/react';
import { Provider } from 'jotai';
import {
  useSelectedNodeData,
  useSelectedNodeIds,
  useSetSelectedNodeData,
  useSetSelectedNodeIds,
  useSetSelectedOperatorLabel,
} from '@quent/hooks';
import { QueryToolbar } from './QueryToolbar';

function ToolbarHarness() {
  const selectedNodeIds = useSelectedNodeIds();
  const selectedNodeData = useSelectedNodeData();
  const setSelectedNodeIds = useSetSelectedNodeIds();
  const setSelectedNodeData = useSetSelectedNodeData();
  const setSelectedOperatorLabel = useSetSelectedOperatorLabel();

  useEffect(() => {
    setSelectedNodeIds(new Set(['parent', 'child']));
    setSelectedOperatorLabel('Parent operator');
    setSelectedNodeData({
      nodeId: 'parent',
      label: 'Parent operator',
      operationType: 'logical',
      statistics: [],
    });
  }, [setSelectedNodeData, setSelectedNodeIds, setSelectedOperatorLabel]);

  return (
    <>
      <QueryToolbar />
      <span data-testid="selected-count">{selectedNodeIds.size}</span>
      <span data-testid="selected-details">{selectedNodeData?.nodeId ?? 'none'}</span>
    </>
  );
}

describe('QueryToolbar', () => {
  it('clears the full operator selection and pinned details', async () => {
    render(
      <Provider>
        <ToolbarHarness />
      </Provider>
    );

    fireEvent.click(await screen.findByRole('button', { name: 'Clear operator filter' }));

    expect(screen.getByTestId('selected-count')).toHaveTextContent('0');
    expect(screen.getByTestId('selected-details')).toHaveTextContent('none');
  });
});
