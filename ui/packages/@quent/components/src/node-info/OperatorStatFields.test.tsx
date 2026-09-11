// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen } from '@testing-library/react';
import { OperatorStatFields } from './OperatorStatFields';

describe('OperatorStatFields', () => {
  it('renders ordered information, observations, and typed port relations', () => {
    render(
      <OperatorStatFields
        operator={{
          nodeId: 'join-1',
          label: 'Join',
          operationType: 'join',
          statistics: [],
          information: [
            { heading: 'Execution', items: [{ key: 'output_rows', value: 42 }] },
            { heading: 'Join', items: [{ key: 'selection_reason', value: 'smaller' }] },
          ],
          portRelations: [{ portId: 'port-1', role: 'build' }],
          observations: [
            {
              timeSeconds: 0.125,
              kind: 'join_build_selected',
              attributes: [{ key: 'build_input_index', value: 1 }],
              portRelations: [{ portId: 'port-1', role: 'build' }],
            },
          ],
        }}
      />
    );

    expect(screen.getByText('Execution')).toBeInTheDocument();
    expect(screen.getByText('Join')).toBeInTheDocument();
    expect(screen.getByText('selection reason:')).toBeInTheDocument();
    expect(screen.getByText('Port relations')).toBeInTheDocument();
    expect(screen.getByText('join_build_selected')).toBeInTheDocument();
    expect(screen.getByText('0.125000 s')).toBeInTheDocument();
    expect(screen.getAllByText('port-1')).toHaveLength(2);
  });
});
