// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { renderHook } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { flattenStatistics, type DAGNode } from '@quent/utils';
import { useOperatorStatFields } from './useDagControls';

describe('useOperatorStatFields', () => {
  it('offers only numeric related-operator stats for grouped nodes', () => {
    const node: DAGNode = {
      id: 'logical',
      label: 'logical',
      type: 'operator',
      metadata: {
        operatorStatistics: { statistics: [], fields: [] },
        relatedOperatorStatistics: [
          {
            statistics: [
              { key: 'rows', value: 4 },
              { key: 'kind', value: 'scan' },
            ],
            fields: flattenStatistics([
              { key: 'rows', value: 4 },
              { key: 'kind', value: 'scan' },
            ]),
          },
        ],
      },
    };
    const { result } = renderHook(() => useOperatorStatFields([node]));
    expect(result.current.map(f => f.key)).toEqual(['rows']);
  });

  it('keeps a node’s own categorical stats', () => {
    const node: DAGNode = {
      id: 'physical',
      label: 'physical',
      type: 'operator',
      metadata: {
        operatorStatistics: {
          statistics: [{ key: 'kind', value: 'scan' }],
          fields: flattenStatistics([{ key: 'kind', value: 'scan' }]),
        },
        relatedOperatorStatistics: [],
      },
    };
    const { result } = renderHook(() => useOperatorStatFields([node]));
    expect(result.current.map(f => f.key)).toEqual(['kind']);
  });
});
