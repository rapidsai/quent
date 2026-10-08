// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { ColorResolver, FiniteStateMachine, QueryEngineFsm } from '@quent/utils';
import {
  LONG_ENTITIES_ROW_TYPE,
  longEntitiesRowId,
  resourceIdFromLongEntitiesRowId,
} from '@quent/utils';
import { stackIntervalsIntoRows } from '../gantt-chart/utils';
import type { LongEntityEntry, LongEntitySegment } from './types';

export { LONG_ENTITIES_ROW_TYPE, longEntitiesRowId, resourceIdFromLongEntitiesRowId };

/**
 * Convert an FSM's consecutive transition pairs into state-colored segments.
 * Each pair defines the time range of the state entered by the first
 * transition (identical semantics to timeline marks). Zero-duration spans are
 * dropped.
 */
function buildSegments(
  fsm: FiniteStateMachine,
  colorFsm: (stateName: string) => string,
  resourceIdsForFilter?: ReadonlySet<string> | null
): LongEntitySegment[] {
  return (fsm.transitions ?? [])
    .slice(0, -1)
    .map((transition, i): LongEntitySegment | null => {
      const next = fsm.transitions[i + 1];
      if (!next) {
        return null;
      }
      if (
        resourceIdsForFilter != null &&
        !transition.usages?.some(usage => resourceIdsForFilter.has(usage.resource))
      ) {
        return null;
      }
      const startMs = transition.timestamp * 1000;
      const endMs = next.timestamp * 1000;
      if (endMs <= startMs) {
        return null;
      }
      return {
        stateName: transition.name,
        startMs,
        endMs,
        color: colorFsm(transition.name),
        // Tolerate responses from servers predating attributes.
        ...((transition.attributes?.length ?? 0) > 0 && { attributes: transition.attributes }),
        ...((transition.derived_attributes?.length ?? 0) > 0 && {
          derivedAttributes: transition.derived_attributes,
        }),
      };
    })
    .filter((s): s is LongEntitySegment => s != null);
}

/**
 * Convert entity-list FSMs into compactly stacked Gantt entries.
 *
 * Each entity becomes one bar spanning its first→last transition, subdivided
 * into state-colored segments. Non-overlapping entities share a row via the
 * greedy first-fit packing shared with the operator Gantt.
 */
export function buildLongEntityEntries(
  items: QueryEngineFsm[],
  colorFsmState: ColorResolver,
  resourceIdsForFilter?: ReadonlySet<string> | null,
  selectedOperatorIds?: ReadonlySet<string> | null
): LongEntityEntry[] {
  const hasOperatorFilter = (selectedOperatorIds?.size ?? 0) > 0;
  const entries: LongEntityEntry[] = [];
  for (const fsm of items) {
    const segments = buildSegments(fsm, colorFsmState, resourceIdsForFilter);
    if (segments.length === 0) {
      continue;
    }
    const startMs = segments[0]!.startMs;
    const endMs = segments[segments.length - 1]!.endMs;
    entries.push({
      entityId: fsm.id,
      label: fsm.instance_name || fsm.id,
      typeName: fsm.type_name,
      startMs,
      endMs,
      rowIndex: 0,
      segments,
      ...(hasOperatorFilter && {
        isDimmed: fsm.operator_id == null || !selectedOperatorIds!.has(fsm.operator_id),
      }),
    });
  }

  // With an operator filter active, pack entities matching the filter first so
  // they claim the topmost swimlanes
  const packingOrder = hasOperatorFilter
    ? [...entries].sort((a, b) => Number(!!a.isDimmed) - Number(!!b.isDimmed))
    : entries;

  return stackIntervalsIntoRows(packingOrder);
}

/** Return every entity state whose half-open segment contains the timestamp. */
export function getLongEntitySegmentsAtTimestamp(
  entries: LongEntityEntry[],
  timestampMs: number
): Array<{ entry: LongEntityEntry; segment: LongEntitySegment }> {
  return entries.flatMap(entry => {
    const segment = entry.segments.find(
      candidate => candidate.startMs <= timestampMs && timestampMs < candidate.endMs
    );
    return segment ? [{ entry, segment }] : [];
  });
}
