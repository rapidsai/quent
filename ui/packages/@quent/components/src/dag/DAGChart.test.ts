// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import type { DAGNode } from '@quent/utils';
import { resolveSelectedOperatorsFromNodes } from './dagSelection';

const NODES: DAGNode[] = [
  {
    id: 'logical',
    label: 'Logical join',
    type: 'join',
    metadata: {
      relatedOperatorIds: ['physical-1', 'physical-2'],
      aggregatedStatistics: [{ key: 'rows', value: 30 }],
    },
  },
  {
    id: 'other',
    label: 'Other operator',
    type: 'scan',
  },
];

describe('resolveSelectedOperatorsFromNodes', () => {
  it('reconstructs multiple physical and higher-level selections', () => {
    const resolved = resolveSelectedOperatorsFromNodes(
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
    expect(resolved.selections[0].selectedData.statistics).toEqual([{ key: 'rows', value: 30 }]);
    expect(resolved.unresolvedOperatorIds).toEqual(new Set());
  });

  it('preserves IDs until matching DAG data is available', () => {
    const selectedIds = new Set(['logical', 'physical-1', 'physical-2', 'unknown']);

    const beforeData = resolveSelectedOperatorsFromNodes([], selectedIds);
    expect(beforeData.selections).toEqual([]);
    expect(beforeData.unresolvedOperatorIds).toEqual(selectedIds);

    const afterData = resolveSelectedOperatorsFromNodes(NODES, selectedIds);
    expect(afterData.selections.map(selection => selection.selectionId)).toEqual(['logical']);
    expect(afterData.unresolvedOperatorIds).toEqual(new Set(['unknown']));
  });
});
