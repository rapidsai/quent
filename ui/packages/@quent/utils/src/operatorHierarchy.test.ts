// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import type { Operator } from './types';
import { buildRelatedOperatorIdsById, resolveOperatorSelections } from './operatorHierarchy';

function makeOperator(id: string, label: string, parentOperatorIds: string[] = []): Operator {
  return {
    id,
    plan_id: null,
    parent_operator_ids: parentOperatorIds,
    instance_name: label,
    operator_type_name: null,
    custom_attributes: {},
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

  it('splits a parent when one covered child is deselected', () => {
    const operators = [
      makeOperator('logical', 'Logical'),
      makeOperator('left', 'Left', ['logical']),
      makeOperator('right', 'Right', ['logical']),
    ];

    const selections = resolveOperatorSelections(operators, ['logical', 'right']);

    expect(new Map(selections.map(selection => [selection.selectionId, selection.label]))).toEqual(
      new Map([
        ['right', 'Right'],
        ['logical', 'Logical'],
      ])
    );
  });
});
