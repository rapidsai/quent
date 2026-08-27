// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { FiniteStateMachine, FsmTypeDecl, PaletteTheme } from '@quent/utils';
import { createFsmTypeColorFn } from '@quent/utils';
import { stackIntervalsIntoRows } from '../gantt-chart/utils';
import type { LongEntityEntry, LongEntitySegment } from './types';

/** Row type identifier for synthetic long-entities rows in the resource tree. */
export const LONG_ENTITIES_ROW_TYPE = 'long-entities';
const LONG_ENTITIES_ROW_ID_PREFIX = '__long_entities__';

/** Id used for the synthetic long-entities row under a resource. */
export function longEntitiesRowId(resourceId: string): string {
  return `${LONG_ENTITIES_ROW_ID_PREFIX}${resourceId}`;
}

/** Extract the resource id from a long-entities row id, or null if it is not one. */
export function resourceIdFromLongEntitiesRowId(id: string): string | null {
  return id.startsWith(LONG_ENTITIES_ROW_ID_PREFIX)
    ? id.slice(LONG_ENTITIES_ROW_ID_PREFIX.length)
    : null;
}

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
  return fsm.transitions
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
  items: FiniteStateMachine[],
  fsmTypes: { [key in string]?: FsmTypeDecl } | undefined,
  theme: PaletteTheme,
  resourceIdsForFilter?: ReadonlySet<string> | null
): LongEntityEntry[] {
  const colorFsm = createFsmTypeColorFn(fsmTypes ?? {}, theme);

  const entries: LongEntityEntry[] = [];
  for (const fsm of items) {
    const segments = buildSegments(fsm, colorFsm, resourceIdsForFilter);
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
    });
  }

  return stackIntervalsIntoRows(entries);
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
