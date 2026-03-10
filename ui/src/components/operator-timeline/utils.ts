import type { QueryBundle } from '~quent/types/QueryBundle';
import type { EntityRef } from '~quent/types/EntityRef';
import type { Operator } from '~quent/types/Operator';
import type { OperatorActiveSpanEntry } from './types';
import { nanosToMs } from '@/lib/timeline.utils';

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

  let rowIndex = 0;
  for (const [operatorId, op] of sorted) {
    const span = op.active_span;
    if (span == null) continue;

    const { startMs, endMs } = spanToMs(span, startTimeNs);
    const typeName = op.operator_type_name ?? '';
    const label = op.instance_name ?? op.operator_type_name ?? operatorId.slice(0, 8);

    entries.push({
      operatorId,
      label,
      typeName,
      startMs,
      endMs,
      rowIndex: rowIndex++,
    });
  }

  return entries;
}
