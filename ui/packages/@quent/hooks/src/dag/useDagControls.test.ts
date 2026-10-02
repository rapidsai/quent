// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { renderHook } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { DAGNode } from '@quent/utils';
import { useOperatorStatFields } from './useDagControls';

const parse = (raw: unknown) =>
  (raw as { stats?: Array<{ key: string; value: unknown }> })?.stats ?? [];

describe('useOperatorStatFields', () => {
  it('offers only numeric related-operator stats for grouped nodes', () => {
    const node: DAGNode = {
      id: 'logical',
      label: 'logical',
      type: 'operator',
      metadata: {
        rawNode: { stats: [] },
        relatedOperators: [
          {
            stats: [
              { key: 'rows', value: 4 },
              { key: 'kind', value: 'scan' },
            ],
          },
        ],
      },
    };
    const { result } = renderHook(() => useOperatorStatFields([node], parse));
    expect(result.current).toEqual(['rows']);
  });

  it('keeps a node’s own categorical stats', () => {
    const node: DAGNode = {
      id: 'physical',
      label: 'physical',
      type: 'operator',
      metadata: { rawNode: { stats: [{ key: 'kind', value: 'scan' }] } },
    };
    const { result } = renderHook(() => useOperatorStatFields([node], parse));
    expect(result.current).toEqual(['kind']);
  });
});
