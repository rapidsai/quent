// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { QueryBundle } from '@quent/utils';
import type { EntityRef } from '@quent/utils';
import type { Operator } from '@quent/utils';
import type { PlanTree } from '@quent/utils';
import type { OperatorActiveSpanEntry } from './types';
import { parseCustomStatistics } from '../lib/queryBundle.utils';
import { stackIntervalsIntoRows } from '../gantt-chart/utils';

/** Row type identifier for synthetic operator-timeline rows in the resource tree. */
export const OPERATOR_TIMELINE_ROW_TYPE = 'operator-timeline';
const OPERATOR_TIMELINE_ROW_ID_PREFIX = '__operator_timeline__';

/** Id used for the synthetic operator-timeline row under a worker resource. */
export function operatorTimelineRowId(workerId: string): string {
  return `${OPERATOR_TIMELINE_ROW_ID_PREFIX}${workerId}`;
}

/** Extract workerId from an operator-timeline row id, or null if not an operator-timeline row. */
export function workerIdFromOperatorTimelineRowId(id: string): string | null {
  return id.startsWith(OPERATOR_TIMELINE_ROW_ID_PREFIX)
    ? id.slice(OPERATOR_TIMELINE_ROW_ID_PREFIX.length)
    : null;
}

/** Collect all non-null worker ids from plan_tree (recursively). */
export function getWorkerIdsFromPlanTree(planTree: PlanTree): string[] {
  const workerIds = new Set<string>();
  function walk(node: PlanTree) {
    if (node.worker != null && node.worker !== '') workerIds.add(node.worker);
    for (const child of node.children ?? []) walk(child);
  }
  walk(planTree);
  return Array.from(workerIds);
}

/** Collect plan ids for which node.worker === workerId (recursively). */
export function getPlanIdsForWorker(planTree: PlanTree, workerId: string): string[] {
  const planIds: string[] = [];
  function walk(node: PlanTree) {
    if (node.worker === workerId) planIds.push(node.id);
    for (const child of node.children ?? []) walk(child);
  }
  walk(planTree);
  return planIds;
}

/** Return every operator whose half-open active span contains the timestamp. */
export function getOperatorsAtTimestamp(
  operators: OperatorActiveSpanEntry[],
  timestampMs: number
): OperatorActiveSpanEntry[] {
  return operators.filter(
    operator => operator.startMs <= timestampMs && timestampMs < operator.endMs
  );
}

/**
 * SpanSec from the API is in seconds relative to query start.
 * Returns ms offsets relative to query start (no absolute epoch base) so the
 * chart x-domain stays float64-exact.
 */
export function spanToMs(span: { start: number; end: number }): {
  startMs: number;
  endMs: number;
} {
  const startMs = span.start * 1_000;
  const endMs = span.end * 1_000;
  return { startMs, endMs };
}

function buildOperatorActiveSpanEntry(
  operatorId: string,
  op: Operator,
  fallbackPlanId?: string
): OperatorActiveSpanEntry | null {
  const span = op.active_span;
  if (span == null) return null;

  const { startMs, endMs } = spanToMs(span);
  const typeName = op.operator_type_name ?? '';
  const label = op.instance_name ?? op.operator_type_name ?? operatorId.slice(0, 8);

  return {
    operatorId,
    label,
    typeName,
    startMs,
    endMs,
    rowIndex: 0,
    planId: op.plan_id ?? fallbackPlanId ?? '',
    statistics: parseCustomStatistics(op),
  };
}

/**
 * Extract operators that have a non-null active_span and convert to chart entries.
 * When planId is provided (non-empty), only operators belonging to that plan are included.
 * Order is stable (by operator id) so row indices are deterministic.
 */
export function operatorsWithActiveSpans(
  queryBundle: QueryBundle<EntityRef>,
  planId?: string | null
): OperatorActiveSpanEntry[] {
  const operators = queryBundle.entities.operators;
  if (!operators) return [];
  if (planId == null || planId === '') return [];

  const entries: OperatorActiveSpanEntry[] = [];
  const sorted = Object.entries(operators)
    .filter((entry): entry is [string, Operator] => entry[1] != null)
    .filter(([, op]) => op.plan_id === planId)
    .sort(([a], [b]) => a.localeCompare(b));

  for (const [operatorId, op] of sorted) {
    const entry = buildOperatorActiveSpanEntry(operatorId, op, planId);
    if (entry) entries.push(entry);
  }

  return stackIntervalsIntoRows(entries);
}

/**
 * Extract operators with active spans for a given worker.
 * Includes operators whose plan_id is in the set of plan ids for that worker (from plan_tree).
 * Order is stable (by operator id).
 */
export function operatorsWithActiveSpansForWorker(
  queryBundle: QueryBundle<EntityRef>,
  workerId: string
): OperatorActiveSpanEntry[] {
  const operators = queryBundle.entities.operators;
  if (!operators) return [];

  const planIds = new Set(getPlanIdsForWorker(queryBundle.plan_tree, workerId));
  if (planIds.size === 0) return [];

  const entries: OperatorActiveSpanEntry[] = [];
  const sorted = Object.entries(operators)
    .filter((entry): entry is [string, Operator] => entry[1] != null)
    .filter(([, op]) => op.plan_id != null && planIds.has(op.plan_id))
    .sort(([a], [b]) => a.localeCompare(b));

  for (const [operatorId, op] of sorted) {
    const entry = buildOperatorActiveSpanEntry(operatorId, op);
    if (entry) entries.push(entry);
  }

  return stackIntervalsIntoRows(entries);
}
