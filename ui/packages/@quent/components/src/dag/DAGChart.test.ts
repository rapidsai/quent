// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import type { DAGNode } from '@quent/utils';
import { resolveInspectedNodeData } from './dagSelection';

const NODES: DAGNode[] = [
  {
    id: 'logical',
    label: 'Logical join',
    type: 'join',
    metadata: { relatedOperatorIds: ['physical-1', 'physical-2'] },
  },
  {
    id: 'other',
    label: 'Other operator',
    type: 'scan',
  },
];

describe('resolveInspectedNodeData', () => {
  it('resolves the primary operator from a hydrated grouped selection', () => {
    expect(
      resolveInspectedNodeData(NODES, new Set(['logical', 'physical-1', 'physical-2']))
    ).toMatchObject({
      nodeId: 'logical',
      label: 'Logical join',
      operationType: 'join',
    });
  });

  it('does not inspect an ambiguous selection', () => {
    expect(resolveInspectedNodeData(NODES, new Set(['logical', 'other']))).toBeNull();
  });
});
