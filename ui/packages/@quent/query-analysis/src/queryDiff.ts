// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { isNumericValue, unwrapTaggedValue } from '@quent/utils';
import type { EntityRef, QueryBundle } from '@quent/utils';

type NumericValue = number | bigint;
export type SerializedQueryDiffValue = number | string;

export type QueryDiffScope = 'logical' | 'physical';

export interface QueryDiffRow {
  scope: QueryDiffScope;
  operatorType: string;
  metric: string;
  quantity: string | null;
  baseline: SerializedQueryDiffValue | null;
  candidate: SerializedQueryDiffValue | null;
  delta: SerializedQueryDiffValue | null;
  deltaPercent: number | null;
}

export type QueryDiffSummary = Omit<QueryDiffRow, 'operatorType'>;

export interface QueryDiffResult {
  baseline: {
    source?: string;
    engineId: string | null;
    queryId: string;
    durationSeconds: number;
  };
  comparisons: QueryDiffComparison[];
  metrics: string[];
  limitations: string[];
}

export interface QueryDiffComparison {
  candidate: {
    source?: string;
    engineId: string | null;
    queryId: string;
    durationSeconds: number;
  };
  summary: QueryDiffSummary[];
  rows: QueryDiffRow[];
}

export interface QueryDiffBundle {
  source?: string;
  engineId?: string | null;
  bundle: QueryBundle<EntityRef>;
}

export interface QueryDiffOptions {
  metrics?: readonly string[];
}

interface AggregatedMetric {
  scope: QueryDiffScope;
  operatorType: string;
  metric: string;
  quantity: string | null;
  value: NumericValue;
}

interface OperatorGroup {
  scope: QueryDiffScope;
  operatorType: string;
}

export const ACTIVE_SPAN_METRIC = 'active_span_s';

function addNumeric(left: NumericValue, right: NumericValue): NumericValue {
  if (typeof left === 'bigint' && typeof right === 'bigint') {
    return left + right;
  }
  return Number(left) + Number(right);
}

function subtractNumeric(left: NumericValue, right: NumericValue): NumericValue {
  if (typeof left === 'bigint' && typeof right === 'bigint') {
    return left - right;
  }
  return Number(left) - Number(right);
}

function serializeNumeric(value: NumericValue): SerializedQueryDiffValue {
  return typeof value === 'bigint' ? value.toString() : value;
}

function metricKey(
  scope: QueryDiffScope,
  operatorType: string,
  metric: string,
  quantity: string | null
): string {
  return [scope, operatorType, metric, quantity ?? ''].join('\0');
}

function aggregateBundle(bundle: QueryBundle<EntityRef>): Map<string, AggregatedMetric> {
  const aggregated = new Map<string, AggregatedMetric>();
  const add = (
    scope: QueryDiffScope,
    operatorType: string,
    metric: string,
    quantity: string | null,
    value: NumericValue
  ) => {
    const key = metricKey(scope, operatorType, metric, quantity);
    const current = aggregated.get(key);
    aggregated.set(key, {
      scope,
      operatorType,
      metric,
      quantity,
      value: current ? addNumeric(current.value, value) : value,
    });
  };

  const operators = Object.values(bundle.entities.operators)
    .filter(operator => operator !== null && operator !== undefined)
    .sort((left, right) => left.id.localeCompare(right.id));
  const rolledUpLogicalOperatorIds = new Set(
    operators.flatMap(operator =>
      operator.parent_operator_ids.filter(parentId => {
        const parent = bundle.entities.operators[parentId];
        const parentPlan = parent?.plan_id ? bundle.entities.plans[parent.plan_id] : undefined;
        return parentPlan?.worker_id === null;
      })
    )
  );

  for (const operator of operators) {
    const plan = operator.plan_id ? bundle.entities.plans[operator.plan_id] : undefined;
    if (!plan) {
      throw new Error(`Operator "${operator.id}" does not reference an available plan.`);
    }
    const operatorType = operator.operator_type_name ?? '(unknown)';
    if (plan.worker_id === null && rolledUpLogicalOperatorIds.has(operator.id)) {
      continue;
    }
    const groups: OperatorGroup[] =
      plan.worker_id === null
        ? [{ scope: 'logical', operatorType }]
        : [{ scope: 'physical', operatorType }];
    if (plan.worker_id !== null) {
      const logicalParentTypes = [
        ...new Set(
          operator.parent_operator_ids.flatMap(parentId => {
            const parent = bundle.entities.operators[parentId];
            const parentPlan = parent?.plan_id ? bundle.entities.plans[parent.plan_id] : undefined;
            return parent && parentPlan?.worker_id === null
              ? [parent.operator_type_name ?? '(unknown)']
              : [];
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
    const addForGroups = (metric: string, quantity: string | null, value: NumericValue) => {
      for (const group of groups) {
        add(group.scope, group.operatorType, metric, quantity, value);
      }
    };

    if (operator.active_span) {
      const duration = operator.active_span.end - operator.active_span.start;
      if (!Number.isFinite(duration) || duration < 0) {
        throw new Error(`Operator "${operator.id}" has an invalid active span.`);
      }
      addForGroups(ACTIVE_SPAN_METRIC, 'seconds', duration);
    }

    for (const statistic of operator.statistics?.custom_statistics ?? []) {
      if (statistic.value.value === null || statistic.value.value === undefined) {
        continue;
      }
      const value = unwrapTaggedValue(statistic.value.value);
      if (isNumericValue(value)) {
        addForGroups(statistic.value.key, statistic.quantity, value);
      }
    }
  }

  return aggregated;
}

export function commonQueryMetrics(bundles: readonly QueryBundle<EntityRef>[]): string[] {
  if (bundles.length === 0) {
    return [];
  }
  const metricSets = bundles.map(
    bundle => new Set([...aggregateBundle(bundle).values()].map(metric => metric.metric))
  );
  return [...metricSets[0]!]
    .filter(metric => metricSets.every(available => available.has(metric)))
    .sort((left, right) => {
      if (left === ACTIVE_SPAN_METRIC) {
        return -1;
      }
      if (right === ACTIVE_SPAN_METRIC) {
        return 1;
      }
      return left.localeCompare(right);
    });
}

function deltaPercent(baseline: NumericValue, candidate: NumericValue): number | null {
  const baselineNumber = Number(baseline);
  const candidateNumber = Number(candidate);
  if (!Number.isFinite(baselineNumber) || !Number.isFinite(candidateNumber)) {
    return null;
  }
  if (baselineNumber === 0) {
    return candidateNumber === 0 ? 0 : null;
  }
  return ((candidateNumber - baselineNumber) / Math.abs(baselineNumber)) * 100;
}

function compareAggregates(
  baseline: Map<string, AggregatedMetric>,
  candidate: Map<string, AggregatedMetric>,
  metrics?: ReadonlySet<string>
): QueryDiffRow[] {
  const keys = new Set([...baseline.keys(), ...candidate.keys()]);
  return [...keys]
    .map(key => {
      const baselineMetric = baseline.get(key);
      const candidateMetric = candidate.get(key);
      const descriptor = baselineMetric ?? candidateMetric!;
      const baselineValue = baselineMetric?.value ?? null;
      const candidateValue = candidateMetric?.value ?? null;
      const delta =
        baselineValue === null || candidateValue === null
          ? null
          : subtractNumeric(candidateValue, baselineValue);

      return {
        scope: descriptor.scope,
        operatorType: descriptor.operatorType,
        metric: descriptor.metric,
        quantity: descriptor.quantity,
        baseline: baselineValue === null ? null : serializeNumeric(baselineValue),
        candidate: candidateValue === null ? null : serializeNumeric(candidateValue),
        delta: delta === null ? null : serializeNumeric(delta),
        deltaPercent:
          baselineValue === null || candidateValue === null
            ? null
            : deltaPercent(baselineValue, candidateValue),
      } satisfies QueryDiffRow;
    })
    .filter(row => !metrics || metrics.has(row.metric))
    .sort(
      (left, right) =>
        left.scope.localeCompare(right.scope) ||
        left.operatorType.localeCompare(right.operatorType) ||
        (left.metric === ACTIVE_SPAN_METRIC ? -1 : right.metric === ACTIVE_SPAN_METRIC ? 1 : 0) ||
        left.metric.localeCompare(right.metric) ||
        (left.quantity ?? '').localeCompare(right.quantity ?? '')
    );
}

function summarizeAggregates(
  aggregates: ReadonlyMap<string, AggregatedMetric>
): Map<string, AggregatedMetric> {
  const summaries = new Map<string, AggregatedMetric>();
  for (const aggregate of aggregates.values()) {
    const key = metricKey(aggregate.scope, 'All operators', aggregate.metric, aggregate.quantity);
    const current = summaries.get(key);
    summaries.set(key, {
      ...aggregate,
      operatorType: 'All operators',
      value: current ? addNumeric(current.value, aggregate.value) : aggregate.value,
    });
  }
  return summaries;
}

function compareSummaries(
  baseline: ReadonlyMap<string, AggregatedMetric>,
  candidate: ReadonlyMap<string, AggregatedMetric>,
  metrics?: ReadonlySet<string>
): QueryDiffSummary[] {
  return compareAggregates(
    summarizeAggregates(baseline),
    summarizeAggregates(candidate),
    metrics
  ).map(row => ({
    scope: row.scope,
    metric: row.metric,
    quantity: row.quantity,
    baseline: row.baseline,
    candidate: row.candidate,
    delta: row.delta,
    deltaPercent: row.deltaPercent,
  }));
}

export function diffQueryBundles(
  baseline: QueryDiffBundle,
  candidates: readonly QueryDiffBundle[],
  options: QueryDiffOptions = {}
): QueryDiffResult {
  if (candidates.length === 0) {
    throw new Error('At least one candidate query bundle is required.');
  }
  const baselineMetrics = aggregateBundle(baseline.bundle);
  const selectedMetrics = options.metrics ? new Set(options.metrics) : undefined;
  const comparisons = candidates.map(candidate => {
    const candidateMetrics = aggregateBundle(candidate.bundle);
    return {
      candidate: {
        ...(candidate.source ? { source: candidate.source } : {}),
        engineId: candidate.engineId ?? null,
        queryId: candidate.bundle.query_id,
        durationSeconds: candidate.bundle.duration_s,
      },
      summary: compareSummaries(baselineMetrics, candidateMetrics, selectedMetrics),
      rows: compareAggregates(baselineMetrics, candidateMetrics, selectedMetrics),
    };
  });

  return {
    baseline: {
      ...(baseline.source ? { source: baseline.source } : {}),
      engineId: baseline.engineId ?? null,
      queryId: baseline.bundle.query_id,
      durationSeconds: baseline.bundle.duration_s,
    },
    comparisons,
    metrics:
      options.metrics?.slice().sort() ??
      [
        ...new Set(comparisons.flatMap(comparison => comparison.rows.map(row => row.metric))),
      ].sort(),
    limitations: [
      'Operator metrics are summed by type; the result does not align individual operator instances.',
      'active_span_s includes idle gaps and is not CPU or GPU execution time.',
      'A missing metric is shown as unavailable and does not receive a numeric delta.',
      'Logical rows roll physical metrics up through parent_operator_ids; standalone logical operators use direct metrics.',
      'Physical operators with multiple logical parent types use one composite logical type.',
      'Summary totals are separate logical and physical views and must not be added together.',
    ],
  };
}
