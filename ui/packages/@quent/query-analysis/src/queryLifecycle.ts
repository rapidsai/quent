// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { EntityRef, QueryBundle } from '@quent/utils';

export type LifecycleOffset = 'planning_s' | 'executing_s' | 'completed_s';

export interface QueryLifecycleAnalysis {
  bundleStartTimeUnixNs: bigint;
  queryStartTimeUnixNs: bigint | null;
  durationSeconds: number;
  offsetsSeconds: {
    planning: number | null;
    executing: number | null;
    completed: number | null;
  };
  intervalsSeconds: {
    planning: number | null;
    execution: number | null;
    beforePlanning: number | null;
    afterCompletion: number | null;
  };
  complete: boolean;
  missingFields: LifecycleOffset[];
  invalidFields: LifecycleOffset[];
  ordered: boolean;
}

function validOffset(value: number | null): value is number {
  return value !== null && Number.isFinite(value) && value >= 0;
}

function interval(start: number | null, end: number | null): number | null {
  return validOffset(start) && validOffset(end) && end >= start ? end - start : null;
}

export function analyzeQueryLifecycle(bundle: QueryBundle<EntityRef>): QueryLifecycleAnalysis {
  if (!Number.isFinite(bundle.duration_s) || bundle.duration_s <= 0) {
    throw new Error('The query bundle has no positive finite duration.');
  }

  const query = bundle.entities.query;
  const offsets = {
    planning_s: query.planning_s,
    executing_s: query.executing_s,
    completed_s: query.completed_s,
  };
  const entries = Object.entries(offsets) as Array<[LifecycleOffset, number | null]>;
  const missingFields = entries.filter(([, value]) => value === null).map(([name]) => name);
  const invalidFields = entries
    .filter(([, value]) => value !== null && !validOffset(value))
    .map(([name]) => name);
  const planning = offsets.planning_s;
  const executing = offsets.executing_s;
  const completed = offsets.completed_s;
  const ordered =
    (!validOffset(planning) || !validOffset(executing) || planning <= executing) &&
    (!validOffset(executing) || !validOffset(completed) || executing <= completed) &&
    (!validOffset(completed) || completed <= bundle.duration_s);

  return {
    bundleStartTimeUnixNs: bundle.start_time_unix_ns,
    queryStartTimeUnixNs: query.start_unix_ns,
    durationSeconds: bundle.duration_s,
    offsetsSeconds: {
      planning,
      executing,
      completed,
    },
    intervalsSeconds: {
      beforePlanning: validOffset(planning) ? planning : null,
      planning: interval(planning, executing),
      execution: interval(executing, completed),
      afterCompletion:
        validOffset(completed) && completed <= bundle.duration_s
          ? bundle.duration_s - completed
          : null,
    },
    complete: missingFields.length === 0 && invalidFields.length === 0 && ordered,
    missingFields,
    invalidFields,
    ordered,
  };
}
