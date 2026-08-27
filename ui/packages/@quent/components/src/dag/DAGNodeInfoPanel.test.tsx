// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect } from 'react';
import { render, screen } from '@testing-library/react';
import { Provider } from 'jotai';
import { useSetSelectedNodeData } from '@quent/hooks';
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

describe('DAGNodeInfoPanel', () => {
  it('shows statistics for every related child operator', async () => {
    render(
      <Provider>
        <SelectedNode />
      </Provider>
    );

    expect(await screen.findByText('Build hash table')).toBeInTheDocument();
    expect(screen.getByText('Probe hash table')).toBeInTheDocument();
    expect(screen.getByText('build rows:')).toBeInTheDocument();
    expect(screen.getByText('probe rows:')).toBeInTheDocument();
    expect(screen.getByText('physical-1')).toBeInTheDocument();
    expect(screen.getByText('physical-2')).toBeInTheDocument();
  });
});
