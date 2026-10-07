// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import type { Operator } from './types';
import {
  buildRelatedOperatorIdsById,
  resolveSelectedOperatorSelections,
  resolveOperatorSelections,
  toggleOperatorSelection,
} from './operatorHierarchy';

function makeOperator(id: string, label: string, parentOperatorIds: string[] = []): Operator {
  return {
    id,
    plan_id: null,
    parent_operator_ids: parentOperatorIds,
    instance_name: label,
    operator_type_name: null,
    custom_attributes: [],
    statistics: null,
    active_span: null,
  };
}

describe('operator hierarchy', () => {
  it('finds direct and transitive descendants', () => {
    const operators = [
      makeOperator('logical', 'Logical'),
      makeOperator('intermediate', 'Intermediate', ['logical']),
      makeOperator('physical', 'Physical', ['intermediate']),
    ];

    expect(buildRelatedOperatorIdsById(operators).get('logical')).toEqual([
      'intermediate',
      'physical',
    ]);
  });

  it('handles parent cycles without including the root', () => {
    const operators = [
      makeOperator('left', 'Left', ['right']),
      makeOperator('right', 'Right', ['left']),
    ];

    expect(buildRelatedOperatorIdsById(operators).get('left')).toEqual(['right']);
  });

  it('collapses a complete descendant set into its maximal parent', () => {
    const operators = [
      makeOperator('logical', 'Logical'),
      makeOperator('left', 'Left', ['logical']),
      makeOperator('right', 'Right', ['logical']),
    ];

    expect(resolveOperatorSelections(operators, ['logical', 'left', 'right'])).toEqual([
      {
        selectionId: 'logical',
        label: 'Logical',
        operatorIds: new Set(['logical', 'left', 'right']),
      },
    ]);
  });

  it('resolves selected operator data from the global operator list', () => {
    const operators = [
      makeOperator('logical', 'Logical'),
      makeOperator('left', 'Left', ['logical']),
      makeOperator('right', 'Right', ['logical']),
    ];

    expect(resolveSelectedOperatorSelections(operators, ['logical', 'left', 'right'])).toEqual([
      {
        selectionId: 'logical',
        label: 'Logical',
        operatorIds: new Set(['logical', 'left', 'right']),
        selectedData: {
          nodeId: 'logical',
          label: 'Logical',
          operationType: 'operator',
          attributes: [],
          statistics: [],
          relatedOperators: [
            {
              nodeId: 'left',
              label: 'Left',
              operationType: 'operator',
              attributes: [],
              statistics: [],
            },
            {
              nodeId: 'right',
              label: 'Right',
              operationType: 'operator',
              attributes: [],
              statistics: [],
            },
          ],
        },
      },
    ]);
  });

  it('splits a parent when one covered child is deselected', () => {
    const operators = [
      makeOperator('logical', 'Logical'),
      makeOperator('left', 'Left', ['logical']),
      makeOperator('right', 'Right', ['logical']),
    ];

    const selectedIds = new Set(['logical', 'left', 'right']);
    const selections = toggleOperatorSelection(
      operators,
      selectedIds,
      new Map([
        [
          'logical',
          {
            label: 'Logical',
            operatorIds: selectedIds,
          },
        ],
      ]),
      'left'
    );

    expect(selections).toEqual([
      {
        selectionId: 'right',
        label: 'Right',
        operatorIds: new Set(['right']),
      },
    ]);
  });

  it('deselects every selected ancestor of a covered descendant', () => {
    const operators = [
      makeOperator('logical', 'Logical'),
      makeOperator('intermediate', 'Intermediate', ['logical']),
      makeOperator('left', 'Left', ['intermediate']),
      makeOperator('right', 'Right', ['intermediate']),
    ];
    const selectedIds = new Set(['logical', 'intermediate', 'left', 'right']);

    expect(
      toggleOperatorSelection(
        operators,
        selectedIds,
        new Map([
          [
            'logical',
            {
              label: 'Logical',
              operatorIds: selectedIds,
            },
          ],
        ]),
        'left'
      )
    ).toEqual([
      {
        selectionId: 'right',
        label: 'Right',
        operatorIds: new Set(['right']),
      },
    ]);
  });

  it('deselects every descendant when a covered parent is deselected', () => {
    const operators = [
      makeOperator('logical', 'Logical'),
      makeOperator('intermediate', 'Intermediate', ['logical']),
      makeOperator('left', 'Left', ['intermediate']),
      makeOperator('right', 'Right', ['intermediate']),
    ];
    const selectedIds = new Set(['logical', 'intermediate', 'left', 'right']);

    expect(
      toggleOperatorSelection(
        operators,
        selectedIds,
        new Map([
          [
            'logical',
            {
              label: 'Logical',
              operatorIds: selectedIds,
            },
          ],
        ]),
        'intermediate'
      )
    ).toEqual([]);
  });
});
