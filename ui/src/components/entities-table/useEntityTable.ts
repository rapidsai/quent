// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useMemo, useState } from 'react';
import { useEntities, useEntityList } from '@quent/client';
import {
  useSelectedNodeIds,
  useSetSelectedNodeIds,
  useSetSelectedOperatorLabel,
} from '@quent/hooks';
import type { OptionMultiSelectOption, SelectFieldOption } from '@quent/components';
import type { EntityRef, FiniteStateMachine, QueryBundle, SortDir } from '@quent/utils';
import { useDebouncedValue } from '@/hooks/useDebouncedValue';
import type { EntityFilters } from './types';
import {
  activeEntityFilterCount,
  buildEntityRequest,
  defaultEntityFilters,
  entityRows,
  fsmSpan,
  hasNonDefaultEntitySettings,
  normalizePageSize,
  operatorLocationDescription,
  resourceLocationDescription,
  selectedOperatorsLabel,
  validateEntityFilters,
} from './utils';

const FILTER_DEBOUNCE_MS = 300;

interface UseEntityTableParams {
  engineId: string;
  queryId: string;
  queryBundle: QueryBundle<EntityRef>;
}

export function useEntityTable({ engineId, queryId, queryBundle }: UseEntityTableParams) {
  const { entities, duration_s: durationS } = queryBundle;
  const operatorIds = useSelectedNodeIds();
  const setSelectedNodeIds = useSetSelectedNodeIds();
  const setSelectedOperatorLabel = useSetSelectedOperatorLabel();
  const defaults = useMemo(() => defaultEntityFilters(durationS), [durationS]);
  // The "Window (s)" slider is bounded by the query duration, which is often far longer than
  // when entities actually occur. Use the longest-running entity's end time as a tighter,
  // more useful max so the slider isn't mostly dead space.
  const longestEntityQuery = useEntityList({
    engineId,
    queryId,
    window: { start: 0, end: durationS },
    sortKey: 'UsageDuration',
    sortDir: 'Desc',
    maxItems: 1,
  });
  const windowMaxS = useMemo(() => {
    const longestEntity = longestEntityQuery.data?.items[0]?.entity;
    if (!longestEntity) {
      return durationS;
    }
    return Math.min(durationS, Math.max(0, fsmSpan(longestEntity).end));
  }, [longestEntityQuery.data, durationS]);
  const [filters, setFilters] = useState<EntityFilters>(() => defaultEntityFilters(durationS));
  const [page, setPage] = useState(0);
  const [selected, setSelected] = useState<FiniteStateMachine | null>(null);
  const operatorLabel = useCallback(
    (id: string) => {
      const operator = entities.operators[id];
      return operator ? (operator.instance_name ?? operator.operator_type_name ?? id) : id;
    },
    [entities.operators]
  );

  // Reset pagination/selection whenever the operator filter changes, regardless of whether
  // it came from this toolbar or another crossfiltered view (DAG, operator swimlanes, etc).
  useEffect(() => {
    setPage(0);
    setSelected(null);
  }, [operatorIds]);

  const updateFilters = useCallback(
    (patch: Partial<EntityFilters>, options?: { preserveSelection?: boolean }) => {
      setFilters(previous => ({ ...previous, ...patch }));
      setPage(0);
      if (!options?.preserveSelection) {
        setSelected(null);
      }
    },
    []
  );

  const updateSortDir = useCallback((sortDir: SortDir) => {
    setFilters(previous => ({ ...previous, sortDir }));
    setPage(0);
  }, []);

  const applyOperatorSelection = useCallback(
    (nextIds: Set<string>) => {
      setSelectedNodeIds(nextIds);
      setSelectedOperatorLabel(selectedOperatorsLabel(nextIds, operatorLabel));
      setPage(0);
      setSelected(null);
    },
    [operatorLabel, setSelectedNodeIds, setSelectedOperatorLabel]
  );

  const toggleOperator = useCallback(
    (value: string) => {
      const next = new Set(operatorIds);
      if (next.has(value)) {
        next.delete(value);
      } else {
        next.add(value);
      }
      applyOperatorSelection(next);
    },
    [applyOperatorSelection, operatorIds]
  );

  const selectAllOperators = useCallback(
    () => applyOperatorSelection(new Set(Object.keys(entities.operators))),
    [applyOperatorSelection, entities.operators]
  );

  const selectNoOperators = useCallback(
    () => applyOperatorSelection(new Set()),
    [applyOperatorSelection]
  );

  const resetFilters = useCallback(() => {
    setFilters(defaults);
    setSelectedNodeIds(new Set());
    setSelectedOperatorLabel(null);
    setPage(0);
    setSelected(null);
  }, [defaults, setSelectedNodeIds, setSelectedOperatorLabel]);

  const operatorOptions = useMemo<OptionMultiSelectOption[]>(
    () =>
      Object.values(entities.operators)
        .map(operator => ({
          value: operator.id,
          label: operator.instance_name ?? operator.operator_type_name ?? operator.id,
          description: operatorLocationDescription(operator, entities.plans, entities.workers),
        }))
        .sort((a, b) => (a.label ?? '').localeCompare(b.label ?? '')),
    [entities.operators, entities.plans, entities.workers]
  );
  const entityTypeOptions = useMemo<SelectFieldOption[]>(
    () =>
      Object.keys(entities.fsm_types)
        .sort()
        .map(name => ({ value: name })),
    [entities.fsm_types]
  );
  const resourceOptions = useMemo<SelectFieldOption[]>(
    () =>
      Object.values(entities.resources)
        .map(resource => ({
          value: resource.id,
          label: `${resource.instance_name} (${resource.type_name})`,
          description: resourceLocationDescription(
            resource,
            entities.resource_groups,
            entities.workers
          ),
        }))
        .sort((a, b) => a.label.localeCompare(b.label)),
    [entities.resources, entities.resource_groups, entities.workers]
  );
  const resourceLabel = useCallback(
    (id: string) => {
      const resource = entities.resources[id];
      return resource ? `${resource.instance_name} (${resource.type_name})` : id;
    },
    [entities.resources]
  );
  const { errors: validationErrors, invalidFields: invalidFilterFields } = useMemo(
    () => validateEntityFilters(filters),
    [filters]
  );

  // Only text inputs need debouncing. Dropdowns, page, and sort fire immediately.
  const debouncedMinUsageS = useDebouncedValue(filters.minUsageS, FILTER_DEBOUNCE_MS);
  const debouncedWindowStart = useDebouncedValue(filters.windowStart, FILTER_DEBOUNCE_MS);
  const debouncedWindowEnd = useDebouncedValue(filters.windowEnd, FILTER_DEBOUNCE_MS);
  const effectiveFilters = useMemo(
    () => ({
      ...filters,
      minUsageS: debouncedMinUsageS,
      windowStart: debouncedWindowStart,
      windowEnd: debouncedWindowEnd,
    }),
    [filters, debouncedMinUsageS, debouncedWindowStart, debouncedWindowEnd]
  );
  const request = useMemo(
    () => buildEntityRequest({ filters: effectiveFilters, operatorIds, page, queryId, durationS }),
    [durationS, effectiveFilters, operatorIds, page, queryId]
  );
  const query = useEntities({ engineId, request }, { enabled: validationErrors.length === 0 });
  const requestPending = query.isFetching;
  const rows = useMemo(() => entityRows(query.data), [query.data]);
  const pageSize = normalizePageSize(filters.pageSize);
  const total = query.data?.total ?? 0;
  const pageCount = Math.max(1, Math.ceil(total / pageSize));
  const visibleStart = total === 0 ? 0 : page * pageSize + 1;
  const visibleEnd = total === 0 ? 0 : Math.min(total, visibleStart + rows.length - 1);
  const activeFilterCount = activeEntityFilterCount(filters, defaults, operatorIds);

  return {
    filters: {
      values: filters,
      durationS,
      windowMaxS,
      validationErrors,
      invalidFilterFields,
      hasNonDefaultSettings: hasNonDefaultEntitySettings(filters, defaults, activeFilterCount),
      activeFilterCount,
      operatorIds,
      operatorOptions,
      entityTypeOptions,
      resourceOptions,
      update: updateFilters,
      toggleOperator,
      selectAllOperators,
      selectNoOperators,
      updateSortDir,
      reset: resetFilters,
    },
    pagination: {
      page,
      pageCount,
      total,
      visibleStart,
      visibleEnd,
      disabled: requestPending || validationErrors.length > 0,
      setPage,
    },
    query: {
      rows,
      error: query.error,
      isError: query.isError,
      isLoading: query.isLoading,
      requestPending,
      fsmTypes: entities.fsm_types,
    },
    selection: {
      selected,
      setSelected,
    },
    resourceLabel,
    operatorLabel,
  };
}
