// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type {
  DynamicAttribute,
  DynamicValue,
  EntityRef,
  OperatorStatistic,
  PortStatistics,
  QueryBundle,
} from '@quent/utils';

export interface NamedDynamicValue {
  path: string[];
  value: DynamicValue | null;
}

export interface NamedOperatorStatistic extends NamedDynamicValue {
  quantity: string | null;
}

export interface OperatorFactsAnalysis {
  operatorCount: number;
  operators: Array<{
    id: string;
    instanceName: string | null;
    typeName: string | null;
    plan: {
      id: string;
      instanceName: string | null;
      parentPlanId: string | null;
      scope: 'logical' | 'physical';
      worker: {
        id: string;
        instanceName: string | null;
      } | null;
    } | null;
    parentOperatorIds: string[];
    childOperatorIds: string[];
    activeSpan: {
      startSeconds: number;
      endSeconds: number;
      durationSeconds: number | null;
    } | null;
    customAttributes: NamedDynamicValue[];
    statistics: NamedOperatorStatistic[];
    ports: Array<{
      id: string;
      instanceName: string | null;
      statistics: NamedDynamicValue[];
    }>;
  }>;
  integrityIssues: string[];
}

function dynamicValues(values: DynamicAttribute[]): NamedDynamicValue[] {
  return [...values]
    .sort((left, right) => left.key.localeCompare(right.key))
    .map(attribute => ({ path: [attribute.key], value: attribute.value }));
}

function operatorStatistics(values: OperatorStatistic[]): NamedOperatorStatistic[] {
  return [...values]
    .sort((left, right) => left.value.key.localeCompare(right.value.key))
    .map(statistic => ({
      path: [statistic.value.key],
      value: statistic.value.value,
      quantity: statistic.quantity,
    }));
}

function portStatistics(statistics: PortStatistics | null): NamedDynamicValue[] {
  return statistics ? dynamicValues(statistics.custom_statistics) : [];
}

export function analyzeOperatorFacts(
  bundle: QueryBundle<EntityRef>,
  operatorIds: readonly string[] = []
): OperatorFactsAnalysis {
  const { operators, plans, workers, ports } = bundle.entities;
  const selectedIds = [...new Set(operatorIds)].sort();
  for (const id of selectedIds) {
    if (!operators[id]) {
      throw new Error(`Operator "${id}" is not present in the query bundle.`);
    }
  }
  const selected = (
    selectedIds.length > 0 ? selectedIds.map(id => operators[id]!) : Object.values(operators)
  ).sort((left, right) => left.id.localeCompare(right.id));
  const childIdsByParent = new Map<string, string[]>();
  for (const operator of Object.values(operators)) {
    for (const parentId of operator.parent_operator_ids) {
      const childIds = childIdsByParent.get(parentId) ?? [];
      childIds.push(operator.id);
      childIdsByParent.set(parentId, childIds);
    }
  }
  for (const ids of childIdsByParent.values()) {
    ids.sort();
  }
  const portsByOperator = new Map<string, (typeof ports)[string][]>();
  for (const port of Object.values(ports)) {
    if (port.operator_id) {
      const operatorPorts = portsByOperator.get(port.operator_id) ?? [];
      operatorPorts.push(port);
      portsByOperator.set(port.operator_id, operatorPorts);
    }
  }
  for (const operatorPorts of portsByOperator.values()) {
    operatorPorts.sort((left, right) => left.id.localeCompare(right.id));
  }
  const issues: string[] = [];
  const projected = selected.map(operator => {
    const plan = operator.plan_id ? plans[operator.plan_id] : undefined;
    if (operator.plan_id && !plan) {
      issues.push(`Operator "${operator.id}" references missing plan "${operator.plan_id}".`);
    }
    for (const parentId of operator.parent_operator_ids) {
      if (!operators[parentId]) {
        issues.push(`Operator "${operator.id}" references missing parent operator "${parentId}".`);
      }
    }
    const worker = plan?.worker_id ? workers[plan.worker_id] : undefined;
    if (plan?.worker_id && !worker) {
      issues.push(`Plan "${plan.id}" references missing worker "${plan.worker_id}".`);
    }
    const activeSpan = operator.active_span;
    const duration =
      activeSpan &&
      Number.isFinite(activeSpan.start) &&
      Number.isFinite(activeSpan.end) &&
      activeSpan.end >= activeSpan.start
        ? activeSpan.end - activeSpan.start
        : null;
    if (activeSpan && duration === null) {
      issues.push(`Operator "${operator.id}" has an invalid active span.`);
    }
    return {
      id: operator.id,
      instanceName: operator.instance_name,
      typeName: operator.operator_type_name,
      plan: plan
        ? {
            id: plan.id,
            instanceName: plan.instance_name,
            parentPlanId: plan.parent,
            scope: plan.worker_id === null ? ('logical' as const) : ('physical' as const),
            worker: worker
              ? {
                  id: worker.id,
                  instanceName: worker.instance_name,
                }
              : null,
          }
        : null,
      parentOperatorIds: [...operator.parent_operator_ids].sort(),
      childOperatorIds: childIdsByParent.get(operator.id) ?? [],
      activeSpan: activeSpan
        ? {
            startSeconds: activeSpan.start,
            endSeconds: activeSpan.end,
            durationSeconds: duration,
          }
        : null,
      customAttributes: dynamicValues(operator.custom_attributes),
      statistics: operator.statistics
        ? operatorStatistics(operator.statistics.custom_statistics)
        : [],
      ports: (portsByOperator.get(operator.id) ?? []).map(port => ({
        id: port.id,
        instanceName: port.instance_name,
        statistics: portStatistics(port.statistics),
      })),
    };
  });

  return {
    operatorCount: projected.length,
    operators: projected,
    integrityIssues: [...new Set(issues)].sort(),
  };
}
