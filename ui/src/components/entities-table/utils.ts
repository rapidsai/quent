// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type {
  EntityListRequest,
  EntityListResponse,
  FiniteStateMachine,
  Operator,
  OperatorFilter,
  Plan,
  QueryFilter,
  Resource,
  ResourceGroup,
  SortDir,
  Worker,
} from '@quent/utils';
import type { EntityFilters, EntityTableRow } from './types';

export const DEFAULT_PAGE_SIZE = 50;
export const MAX_PAGE_SIZE = 500;

export const SORT_ASC: SortDir = 'Asc';
export const SORT_DESC: SortDir = 'Desc';

export function defaultEntityFilters(durationS: number): EntityFilters {
  return {
    entityType: null,
    resourceId: null,
    minUsageS: '',
    windowStart: '0',
    windowEnd: String(durationS),
    sortDir: SORT_DESC,
    pageSize: DEFAULT_PAGE_SIZE,
  };
}

export function normalizePageSize(value: number | null): number {
  if (value === null || !Number.isFinite(value)) {
    return DEFAULT_PAGE_SIZE;
  }
  return Math.min(MAX_PAGE_SIZE, Math.max(1, Math.trunc(value)));
}

export type EntityNumberFilterField = 'windowStart' | 'windowEnd' | 'minUsageS';

export interface EntityFilterValidation {
  errors: string[];
  invalidFields: Set<EntityNumberFilterField>;
}

export function validateEntityFilters(filters: EntityFilters): EntityFilterValidation {
  const windowStart = parseOptionalNumber(filters.windowStart);
  const windowEnd = parseOptionalNumber(filters.windowEnd);
  const minUsageS = parseOptionalNumber(filters.minUsageS);
  const errors: string[] = [];
  const invalidFields = new Set<EntityNumberFilterField>();

  if (filters.windowStart.trim() !== '' && windowStart === null) {
    errors.push('Window start must be a number.');
    invalidFields.add('windowStart');
  }
  if (filters.windowEnd.trim() !== '' && windowEnd === null) {
    errors.push('Window end must be a number.');
    invalidFields.add('windowEnd');
  }
  if (filters.minUsageS.trim() !== '' && minUsageS === null) {
    errors.push('Minimum usage must be a number.');
    invalidFields.add('minUsageS');
  }
  if (windowStart !== null && windowStart < 0) {
    errors.push('Window start cannot be negative.');
    invalidFields.add('windowStart');
  }
  if (windowEnd !== null && windowEnd < 0) {
    errors.push('Window end cannot be negative.');
    invalidFields.add('windowEnd');
  }
  if (windowStart !== null && windowEnd !== null && windowStart > windowEnd) {
    errors.push('Window start must not exceed window end.');
    invalidFields.add('windowStart');
    invalidFields.add('windowEnd');
  }
  if (minUsageS !== null && minUsageS < 0) {
    errors.push('Minimum usage cannot be negative.');
    invalidFields.add('minUsageS');
  }

  return { errors, invalidFields };
}

export function buildEntityRequest({
  filters,
  operatorIds,
  page,
  queryId,
  durationS,
}: {
  filters: EntityFilters;
  operatorIds: ReadonlySet<string>;
  page: number;
  queryId: string;
  durationS: number;
}): EntityListRequest<QueryFilter, OperatorFilter> {
  return {
    entry: {
      window: {
        start: parseOptionalNumber(filters.windowStart) ?? 0,
        end: parseOptionalNumber(filters.windowEnd) ?? durationS,
      },
      filter: {
        scope: filters.resourceId ? { Resource: { resource_id: filters.resourceId } } : null,
        entity_type_name: filters.entityType,
        min_usage_s: parseOptionalNumber(filters.minUsageS),
      },
      sort: { key: 'UsageDuration', dir: filters.sortDir },
      page: { max: normalizePageSize(filters.pageSize), page },
      application: { operator_ids: [...operatorIds] },
    },
    app_params: { query_id: queryId },
  };
}

export function entityRows(data: EntityListResponse | undefined): EntityTableRow[] {
  return (data?.items ?? []).map(item => {
    const span = fsmSpan(item.entity);
    return { fsm: item.entity, usageDurationS: item.usage_duration_s, ...span };
  });
}

/** Compares two numeric filter strings by value rather than formatting, since sliders and typed input can format the same value differently (e.g. "0" vs "0.000"). */
function numericFilterValueChanged(value: string, defaultValue: string): boolean {
  return parseOptionalNumber(value) !== parseOptionalNumber(defaultValue);
}

export function activeEntityFilterCount(
  filters: EntityFilters,
  defaults: EntityFilters,
  operatorIds: ReadonlySet<string>
): number {
  return [
    operatorIds.size > 0,
    filters.entityType !== null,
    filters.resourceId !== null,
    filters.minUsageS !== '',
    numericFilterValueChanged(filters.windowStart, defaults.windowStart),
    numericFilterValueChanged(filters.windowEnd, defaults.windowEnd),
  ].filter(Boolean).length;
}

export function hasNonDefaultEntitySettings(
  filters: EntityFilters,
  defaults: EntityFilters,
  activeFilterCount: number
): boolean {
  return (
    activeFilterCount > 0 ||
    filters.sortDir !== defaults.sortDir ||
    filters.pageSize !== defaults.pageSize
  );
}

export function parseOptionalNumber(value: string): number | null {
  if (value.trim() === '') {
    return null;
  }
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

/** Builds the crossfilter chip label (e.g. shown in QueryToolbar) for a set of selected operators. */
export function selectedOperatorsLabel(
  operatorIds: ReadonlySet<string>,
  operatorLabel: (id: string) => string
): string | null {
  if (operatorIds.size === 0) {
    return null;
  }
  if (operatorIds.size === 1) {
    return operatorLabel([...operatorIds][0]);
  }
  return `${operatorIds.size} operators`;
}

/** Builds a "Plan / Worker" subtitle so operators sharing the same name can be told apart. */
export function operatorLocationDescription(
  operator: Operator,
  plans: Record<string, Plan>,
  workers: Record<string, Worker>
): string | undefined {
  const plan = operator.plan_id ? plans[operator.plan_id] : undefined;
  if (!plan) {
    return undefined;
  }
  const planLabel = plan.instance_name ?? plan.id;
  const worker = plan.worker_id ? workers[plan.worker_id] : undefined;
  const workerLabel = worker ? (worker.instance_name ?? worker.id) : null;
  return workerLabel ? `Plan: ${planLabel} · Worker: ${workerLabel}` : `Plan: ${planLabel}`;
}

export function resourceLocationDescription(
  resource: Resource,
  resourceGroups: Record<string, ResourceGroup>,
  workers: Record<string, Worker>
): string | undefined {
  const visited = new Set<string>();
  let groupId: string | null = resource.parent_group_id;
  while (groupId && !visited.has(groupId)) {
    visited.add(groupId);
    const worker = workers[groupId];
    if (worker) {
      return `Worker: ${worker.instance_name ?? worker.id}`;
    }
    groupId = resourceGroups[groupId]?.parent_group_id ?? null;
  }
  return undefined;
}

export function fsmSpan(fsm: FiniteStateMachine): { start: number; end: number } {
  let start = Infinity;
  let end = -Infinity;
  for (const transition of fsm.transitions) {
    if (transition.timestamp < start) {
      start = transition.timestamp;
    }
    if (transition.timestamp > end) {
      end = transition.timestamp;
    }
  }
  return fsm.transitions.length === 0 ? { start: 0, end: 0 } : { start, end };
}
