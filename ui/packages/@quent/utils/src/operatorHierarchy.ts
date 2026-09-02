// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { Operator } from './types';
import type { OperatorSelectionInput } from './operatorTypes';

export function getOperatorDisplayLabel(operator: Operator): string {
  return operator.instance_name ?? operator.operator_type_name ?? operator.id;
}

export function buildRelatedOperatorIdsById(
  operators: readonly Operator[],
  rootOperatorIds: Iterable<string> = operators.map(operator => operator.id)
): Map<string, string[]> {
  const childrenByParentId = new Map<string, string[]>();
  for (const operator of operators) {
    for (const parentId of operator.parent_operator_ids ?? []) {
      const children = childrenByParentId.get(parentId) ?? [];
      children.push(operator.id);
      childrenByParentId.set(parentId, children);
    }
  }

  const relatedById = new Map<string, string[]>();
  for (const operatorId of rootOperatorIds) {
    const related = new Set<string>();
    const stack = [...(childrenByParentId.get(operatorId) ?? [])];
    while (stack.length > 0) {
      const childId = stack.pop()!;
      if (childId === operatorId || related.has(childId)) {
        continue;
      }
      related.add(childId);
      stack.push(...(childrenByParentId.get(childId) ?? []));
    }
    relatedById.set(operatorId, [...related].sort());
  }

  return relatedById;
}

export function resolveOperatorSelections(
  operators: readonly Operator[],
  selectedOperatorIds: Iterable<string>
): OperatorSelectionInput[] {
  const selectedIds = [...new Set(selectedOperatorIds)];
  const remainingIds = new Set(selectedIds);
  const operatorsById = new Map(operators.map(operator => [operator.id, operator]));
  const relatedById = buildRelatedOperatorIdsById(operators, selectedIds);
  const orderById = new Map(selectedIds.map((id, index) => [id, index]));
  const candidates = selectedIds
    .flatMap(id => {
      const operator = operatorsById.get(id);
      if (!operator) {
        return [];
      }
      return [
        {
          operator,
          operatorIds: new Set([id, ...(relatedById.get(id) ?? [])]),
        },
      ];
    })
    .sort(
      (left, right) =>
        right.operatorIds.size - left.operatorIds.size ||
        (orderById.get(left.operator.id) ?? 0) - (orderById.get(right.operator.id) ?? 0)
    );

  const selections: OperatorSelectionInput[] = [];
  for (const candidate of candidates) {
    if (![...candidate.operatorIds].every(id => remainingIds.has(id))) {
      continue;
    }
    selections.push({
      selectionId: candidate.operator.id,
      label: getOperatorDisplayLabel(candidate.operator),
      operatorIds: candidate.operatorIds,
    });
    for (const id of candidate.operatorIds) {
      remainingIds.delete(id);
    }
  }

  for (const id of selectedIds) {
    if (!remainingIds.delete(id)) {
      continue;
    }
    const operator = operatorsById.get(id);
    selections.push({
      selectionId: id,
      label: operator ? getOperatorDisplayLabel(operator) : id,
      operatorIds: new Set([id]),
    });
  }

  return selections;
}
