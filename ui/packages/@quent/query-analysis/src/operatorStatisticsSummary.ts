// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { EntityRef, Operator, QuantitySpec, QueryBundle } from '@quent/utils';

type NumericValue = number | bigint;
export type SerializedNumericValue = number | string;
export type StatisticScope = 'logical' | 'physical';
export type StatisticPath = Array<[name: string, index: number]>;
export type SkippedReason = 'missing' | 'non-numeric' | 'list' | 'non-finite' | 'out-of-range';

interface OperatorGroup {
  scope: StatisticScope;
  operatorType: string;
}

interface StatisticObservation {
  operator: Operator;
  group: OperatorGroup;
  path: StatisticPath;
  quantity: string | null;
  value?: NumericValue;
  reason?: SkippedReason;
}

export interface ExtremeSource {
  operatorId: string;
  operatorName: string | null;
  planId: string;
  planName: string | null;
  workerId: string | null;
  workerName: string | null;
}

export interface OperatorStatisticsSummary {
  aggregates: Array<{
    scope: StatisticScope;
    operatorType: string;
    statisticPath: StatisticPath;
    quantity: string | null;
    quantitySpec: QuantitySpec | null;
    operatorCount: number;
    numericValueCount: number;
    missingValueCount: number;
    nonNumericValueCount: number;
    sum: SerializedNumericValue;
    minimum: SerializedNumericValue;
    maximum: SerializedNumericValue;
    mean: number;
    median: number;
    populationVariance: number;
    populationStandardDeviation: number;
    percentile25: number;
    percentile75: number;
    minimumSources: ExtremeSource[];
    maximumSources: ExtremeSource[];
    approximateFloatingPoint: boolean;
  }>;
  skippedValues: Array<{
    scope: StatisticScope;
    operatorType: string;
    statisticPath: StatisticPath;
    quantity: string | null;
    operatorId: string;
    reason: SkippedReason;
  }>;
  undeclaredQuantities: string[];
  groupingIssues: string[];
}

function pathKey(path: StatisticPath): string {
  return JSON.stringify(path);
}

function groupKey(group: OperatorGroup): string {
  return `${group.scope}\0${group.operatorType}`;
}

function aggregateKey(group: OperatorGroup, path: StatisticPath, quantity: string | null): string {
  return `${groupKey(group)}\0${pathKey(path)}\0${quantity ?? ''}`;
}

function groupOperators(bundle: QueryBundle<EntityRef>): {
  memberships: Map<string, OperatorGroup[]>;
  operatorsByGroup: Map<string, Set<string>>;
  issues: string[];
} {
  const { operators, plans } = bundle.entities;
  const orderedOperators = Object.values(operators).sort((left, right) =>
    left.id.localeCompare(right.id)
  );
  const rolledUpLogicalOperatorIds = new Set(
    orderedOperators.flatMap(operator =>
      operator.parent_operator_ids.filter(parentId => {
        const parent = operators[parentId];
        const parentPlan = parent?.plan_id ? plans[parent.plan_id] : undefined;
        return parentPlan?.worker_id === null;
      })
    )
  );
  const memberships = new Map<string, OperatorGroup[]>();
  const operatorsByGroup = new Map<string, Set<string>>();
  const issues: string[] = [];

  for (const operator of orderedOperators) {
    const plan = operator.plan_id ? plans[operator.plan_id] : undefined;
    if (!plan) {
      throw new Error(`Operator "${operator.id}" does not reference an available plan.`);
    }
    const operatorType = operator.operator_type_name ?? '(unknown)';
    const groups: OperatorGroup[] = [];
    if (plan.worker_id === null) {
      if (!rolledUpLogicalOperatorIds.has(operator.id)) {
        groups.push({ scope: 'logical', operatorType });
      }
    } else {
      groups.push({ scope: 'physical', operatorType });
      const logicalParentTypes = [
        ...new Set(
          operator.parent_operator_ids.flatMap(parentId => {
            const parent = operators[parentId];
            if (!parent) {
              issues.push(
                `Operator "${operator.id}" references missing parent operator "${parentId}".`
              );
              return [];
            }
            const parentPlan = parent.plan_id ? plans[parent.plan_id] : undefined;
            return parentPlan?.worker_id === null ? [parent.operator_type_name ?? '(unknown)'] : [];
          })
        ),
      ].sort();
      if (logicalParentTypes.length > 0) {
        groups.push({
          scope: 'logical',
          operatorType: logicalParentTypes.join(', '),
        });
      }
    }
    memberships.set(operator.id, groups);
    for (const group of groups) {
      const ids = operatorsByGroup.get(groupKey(group)) ?? new Set<string>();
      ids.add(operator.id);
      operatorsByGroup.set(groupKey(group), ids);
    }
  }
  return { memberships, operatorsByGroup, issues: [...new Set(issues)].sort() };
}

function isDynamicAttribute(value: unknown): value is { key: string; value: unknown } {
  return (
    typeof value === 'object' &&
    value !== null &&
    !Array.isArray(value) &&
    typeof (value as { key?: unknown }).key === 'string' &&
    'value' in value
  );
}

function appendStruct(
  attributes: unknown[],
  path: StatisticPath,
  visit: (value: unknown, path: StatisticPath) => void
): void {
  const indexes = new Map<string, number>();
  for (const attribute of attributes) {
    if (!isDynamicAttribute(attribute)) {
      visit(attribute, path);
      continue;
    }
    const index = indexes.get(attribute.key) ?? 0;
    indexes.set(attribute.key, index + 1);
    visit(attribute.value, [...path, [attribute.key, index]]);
  }
}

function flattenStatistic(
  value: unknown,
  rootPath: StatisticPath,
  observe: (path: StatisticPath, value?: NumericValue, reason?: SkippedReason) => void
): void {
  const visit = (current: unknown, path: StatisticPath): void => {
    if (current === null || current === undefined) {
      observe(path, undefined, 'missing');
      return;
    }
    if (typeof current === 'bigint') {
      if (!Number.isFinite(Number(current))) {
        observe(path, undefined, 'out-of-range');
      } else {
        observe(path, current);
      }
      return;
    }
    if (typeof current === 'number') {
      if (Number.isFinite(current)) {
        observe(path, current);
      } else {
        observe(path, undefined, 'non-finite');
      }
      return;
    }
    if (typeof current === 'string' || typeof current === 'boolean') {
      observe(path, undefined, 'non-numeric');
      return;
    }
    if (Array.isArray(current)) {
      if (current.every(isDynamicAttribute)) {
        appendStruct(current, path, visit);
      } else {
        observe(path, undefined, 'list');
      }
      return;
    }
    if (typeof current === 'object') {
      const entries = Object.entries(current as Record<string, unknown>);
      if (entries.length !== 1) {
        observe(path, undefined, 'non-numeric');
        return;
      }
      const [tag, inner] = entries[0]!;
      if (tag === 'Struct') {
        if (Array.isArray(inner)) {
          appendStruct(inner, path, visit);
        } else {
          observe(path, undefined, 'non-numeric');
        }
        return;
      }
      if (tag === 'List' || Array.isArray(inner)) {
        observe(path, undefined, 'list');
        return;
      }
      visit(inner, path);
      return;
    }
    observe(path, undefined, 'non-numeric');
  };
  visit(value, rootPath);
}

function compareNumeric(left: NumericValue, right: NumericValue): number {
  if (typeof left === 'bigint' && typeof right === 'bigint') {
    return left < right ? -1 : left > right ? 1 : 0;
  }
  return Number(left) - Number(right);
}

function serializeNumeric(value: NumericValue): SerializedNumericValue {
  return typeof value === 'bigint' ? value.toString() : value;
}

function sumNumbers(values: readonly number[]): number {
  let sum = 0;
  let compensation = 0;
  for (const value of values) {
    const adjusted = value - compensation;
    const next = sum + adjusted;
    compensation = next - sum - adjusted;
    sum = next;
  }
  return sum;
}

function percentile(sorted: readonly number[], fraction: number): number {
  const position = (sorted.length - 1) * fraction;
  const lower = Math.floor(position);
  const upper = Math.ceil(position);
  if (lower === upper) {
    return sorted[lower]!;
  }
  const weight = position - lower;
  return sorted[lower]! * (1 - weight) + sorted[upper]! * weight;
}

function source(bundle: QueryBundle<EntityRef>, operator: Operator): ExtremeSource {
  const plan = operator.plan_id ? bundle.entities.plans[operator.plan_id] : undefined;
  if (!plan) {
    throw new Error(`Operator "${operator.id}" does not reference an available plan.`);
  }
  const worker = plan.worker_id ? bundle.entities.workers[plan.worker_id] : undefined;
  return {
    operatorId: operator.id,
    operatorName: operator.instance_name,
    planId: plan.id,
    planName: plan.instance_name,
    workerId: plan.worker_id,
    workerName: worker?.instance_name ?? null,
  };
}

export function summarizeOperatorStatistics(
  bundle: QueryBundle<EntityRef>
): OperatorStatisticsSummary {
  const { memberships, operatorsByGroup, issues } = groupOperators(bundle);
  const observations: StatisticObservation[] = [];
  const undeclaredQuantities = new Set<string>();
  const operators = Object.values(bundle.entities.operators).sort((left, right) =>
    left.id.localeCompare(right.id)
  );

  for (const operator of operators) {
    const occurrences = new Map<string, number>();
    for (const statistic of [...(operator.statistics?.custom_statistics ?? [])].sort(
      (left, right) => left.value.key.localeCompare(right.value.key)
    )) {
      const statisticName = statistic.value.key;
      const occurrence = occurrences.get(statisticName) ?? 0;
      occurrences.set(statisticName, occurrence + 1);
      if (statistic.quantity && !bundle.quantity_specs[statistic.quantity]) {
        undeclaredQuantities.add(statistic.quantity);
      }
      flattenStatistic(
        statistic.value.value,
        [[statisticName, occurrence]],
        (path, value, reason) => {
          for (const group of memberships.get(operator.id) ?? []) {
            observations.push({
              operator,
              group,
              path,
              quantity: statistic.quantity,
              ...(value === undefined ? {} : { value }),
              ...(reason ? { reason } : {}),
            });
          }
        }
      );
    }
  }

  const observationsByAggregate = new Map<string, StatisticObservation[]>();
  for (const observation of observations) {
    const key = aggregateKey(observation.group, observation.path, observation.quantity);
    const values = observationsByAggregate.get(key) ?? [];
    values.push(observation);
    observationsByAggregate.set(key, values);
  }
  const aggregates = [...observationsByAggregate.values()]
    .filter(group => group.some(observation => observation.value !== undefined))
    .map(group => {
      const descriptor = group[0]!;
      const numeric = group
        .filter(
          (observation): observation is StatisticObservation & { value: NumericValue } =>
            observation.value !== undefined
        )
        .sort(
          (left, right) =>
            compareNumeric(left.value, right.value) ||
            left.operator.id.localeCompare(right.operator.id)
        );
      const numericValues = numeric.map(observation => observation.value);
      const numbers = numericValues.map(Number).sort((left, right) => left - right);
      const allBigInt = numericValues.every(value => typeof value === 'bigint');
      const hasUnsafeBigInt = numericValues.some(
        value => typeof value === 'bigint' && !Number.isSafeInteger(Number(value))
      );
      const sum: NumericValue = allBigInt
        ? (numericValues as bigint[]).reduce((total, value) => total + value, 0n)
        : sumNumbers(numbers);
      const mean = sumNumbers(numbers) / numbers.length;
      const populationVariance =
        sumNumbers(numbers.map(value => (value - mean) ** 2)) / numbers.length;
      const minimum = numeric[0]!.value;
      const maximum = numeric[numeric.length - 1]!.value;
      const declaredOperatorIds = new Set(
        group
          .filter(observation => observation.reason !== 'missing')
          .map(observation => observation.operator.id)
      );
      const typeOperatorIds = operatorsByGroup.get(groupKey(descriptor.group)) ?? new Set<string>();
      const nonNumericValueCount = group.filter(
        observation => observation.value === undefined && observation.reason !== 'missing'
      ).length;
      return {
        scope: descriptor.group.scope,
        operatorType: descriptor.group.operatorType,
        statisticPath: descriptor.path,
        quantity: descriptor.quantity,
        quantitySpec: descriptor.quantity
          ? (bundle.quantity_specs[descriptor.quantity] ?? null)
          : null,
        operatorCount: typeOperatorIds.size,
        numericValueCount: numeric.length,
        missingValueCount: typeOperatorIds.size - declaredOperatorIds.size,
        nonNumericValueCount,
        sum: serializeNumeric(sum),
        minimum: serializeNumeric(minimum),
        maximum: serializeNumeric(maximum),
        mean,
        median: percentile(numbers, 0.5),
        populationVariance,
        populationStandardDeviation: Math.sqrt(populationVariance),
        percentile25: percentile(numbers, 0.25),
        percentile75: percentile(numbers, 0.75),
        minimumSources: numeric
          .filter(observation => compareNumeric(observation.value, minimum) === 0)
          .map(observation => source(bundle, observation.operator)),
        maximumSources: numeric
          .filter(observation => compareNumeric(observation.value, maximum) === 0)
          .map(observation => source(bundle, observation.operator)),
        approximateFloatingPoint: hasUnsafeBigInt,
      };
    })
    .sort(
      (left, right) =>
        left.scope.localeCompare(right.scope) ||
        left.operatorType.localeCompare(right.operatorType) ||
        pathKey(left.statisticPath).localeCompare(pathKey(right.statisticPath)) ||
        (left.quantity ?? '').localeCompare(right.quantity ?? '')
    );
  const skippedValues = observations
    .filter(
      (
        observation
      ): observation is StatisticObservation & {
        reason: SkippedReason;
      } => observation.value === undefined && observation.reason !== undefined
    )
    .map(observation => ({
      scope: observation.group.scope,
      operatorType: observation.group.operatorType,
      statisticPath: observation.path,
      quantity: observation.quantity,
      operatorId: observation.operator.id,
      reason: observation.reason,
    }))
    .sort(
      (left, right) =>
        left.scope.localeCompare(right.scope) ||
        left.operatorType.localeCompare(right.operatorType) ||
        pathKey(left.statisticPath).localeCompare(pathKey(right.statisticPath)) ||
        (left.quantity ?? '').localeCompare(right.quantity ?? '') ||
        left.operatorId.localeCompare(right.operatorId) ||
        left.reason.localeCompare(right.reason)
    );

  return {
    aggregates,
    skippedValues,
    undeclaredQuantities: [...undeclaredQuantities].sort(),
    groupingIssues: issues,
  };
}
