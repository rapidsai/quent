// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import type { DAGNode } from '@quent/utils';
import { resolveInspectedNodeData, resolveInspectedNodeSelections } from './dagSelection';

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

  it('reconstructs multiple physical and higher-level selections', () => {
    const resolved = resolveInspectedNodeSelections(
      NODES,
      new Set(['logical', 'physical-1', 'physical-2', 'other'])
    );

    expect(resolved.selections).toMatchObject([
      {
        selectionId: 'logical',
        label: 'Logical join',
        operatorIds: new Set(['logical', 'physical-1', 'physical-2']),
      },
      {
        selectionId: 'other',
        label: 'Other operator',
        operatorIds: new Set(['other']),
      },
    ]);
    expect(resolved.unresolvedOperatorIds).toEqual(new Set());
  });

  it('preserves IDs until matching DAG data is available', () => {
    const selectedIds = new Set(['logical', 'physical-1', 'physical-2', 'unknown']);

    const beforeData = resolveInspectedNodeSelections([], selectedIds);
    expect(beforeData.selections).toEqual([]);
    expect(beforeData.unresolvedOperatorIds).toEqual(selectedIds);

    const afterData = resolveInspectedNodeSelections(NODES, selectedIds);
    expect(afterData.selections.map(selection => selection.selectionId)).toEqual(['logical']);
    expect(afterData.unresolvedOperatorIds).toEqual(new Set(['unknown']));
  });
});
