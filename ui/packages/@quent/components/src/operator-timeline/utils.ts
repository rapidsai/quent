// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { QueryBundle } from '@quent/utils';
import type { EntityRef } from '@quent/utils';
import type { Operator } from '@quent/utils';
import type { PlanTree } from '@quent/utils';
import type { OperatorActiveSpanEntry } from './types';
import { nanosToMs } from '../lib/timeline.utils';
import { parseCustomStatistics } from '../lib/queryBundle.utils';

/** Clip a rect to bounds (same behavior as ECharts custom-gantt-flight example). */
export function clipRectByRect(
  target: { x: number; y: number; width: number; height: number },
  bounds: { x: number; y: number; width: number; height: number }
): { x: number; y: number; width: number; height: number } | undefined {
  const x = Math.max(target.x, bounds.x);
  const x2 = Math.min(target.x + target.width, bounds.x + bounds.width);
  const y = Math.max(target.y, bounds.y);
  const y2 = Math.min(target.y + target.height, bounds.y + bounds.height);
  if (x2 >= x && y2 >= y) {
    return { x, y, width: x2 - x, height: y2 - y };
  }
  return undefined;
}

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

/**
 * Stack operators into as few rows as possible so that no two bars overlap in the same row.
 * Uses a greedy first-fit by start time: sort by startMs, then assign each bar to the first
 * row where it doesn't overlap the last bar in that row.
 * Mutates entries in place (sets rowIndex) and returns the same array.
 */
export function stackOperatorsIntoRows<
  T extends { startMs: number; endMs: number; rowIndex: number },
>(entries: T[]): T[] {
  if (entries.length === 0) return entries;

  const sorted = [...entries].sort((a, b) => a.startMs - b.startMs || a.endMs - b.endMs);
  /** For each row index, the end time of the rightmost bar in that row (no bar in row extends past this). */
  const rowEndMs: number[] = [];

  for (const entry of sorted) {
    let row = 0;
    while (row < rowEndMs.length && entry.startMs < rowEndMs[row]) {
      row++;
    }
    if (row === rowEndMs.length) {
      rowEndMs.push(entry.endMs);
    } else {
      rowEndMs[row] = Math.max(rowEndMs[row], entry.endMs);
    }
    entry.rowIndex = row;
  }

  return entries;
}

/**
 * SpanSec from the API is in seconds relative to query start (epoch).
 * Convert to absolute ms using query start time.
 */
export function spanToMs(
  span: { start: number; end: number },
  startTimeNs: bigint
): { startMs: number; endMs: number } {
  const startMs = nanosToMs(startTimeNs) + span.start * 1_000;
  const endMs = nanosToMs(startTimeNs) + span.end * 1_000;
  return { startMs, endMs };
}

function buildOperatorActiveSpanEntry(
  operatorId: string,
  op: Operator,
  startTimeNs: bigint,
  fallbackPlanId?: string
): OperatorActiveSpanEntry | null {
  const span = op.active_span;
  if (span == null) return null;

  const { startMs, endMs } = spanToMs(span, startTimeNs);
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
  startTimeNs: bigint,
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
    const entry = buildOperatorActiveSpanEntry(operatorId, op, startTimeNs, planId);
    if (entry) entries.push(entry);
  }

  return stackOperatorsIntoRows(entries);
}

/**
 * Extract operators with active spans for a given worker.
 * Includes operators whose plan_id is in the set of plan ids for that worker (from plan_tree).
 * Order is stable (by operator id).
 */
export function operatorsWithActiveSpansForWorker(
  queryBundle: QueryBundle<EntityRef>,
  startTimeNs: bigint,
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
    const entry = buildOperatorActiveSpanEntry(operatorId, op, startTimeNs);
    if (entry) entries.push(entry);
  }

  return stackOperatorsIntoRows(entries);
}
