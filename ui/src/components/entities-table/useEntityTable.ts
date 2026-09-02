// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useMemo, useRef, type SetStateAction } from 'react';
import { useAtom } from 'jotai';
import { useEntities, useEntityList } from '@quent/client';
import { useSelectedNodeIds } from '@quent/hooks';
import type { SelectFieldOption } from '@quent/components';
import type { EntityRef, FiniteStateMachine, QueryBundle, SortDir } from '@quent/utils';
import { entitiesTableStateAtom } from '@/atoms/entitiesTable';
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
  const selectedNodeIds = useSelectedNodeIds();
  const dagOperatorId = selectedNodeIds.values().next().value ?? null;
  const defaults = useMemo(() => defaultEntityFilters(durationS), [durationS]);
  const [tableState, setTableState] = useAtom(entitiesTableStateAtom);
  const filters = tableState.filters ?? defaults;
  const { manualOperatorOverride, page, selected, selectedEntityId } = tableState;
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
  const previousDagOperatorId = useRef(dagOperatorId);
  const operatorId =
    manualOperatorOverride?.dagOperatorId === dagOperatorId
      ? manualOperatorOverride.value
      : dagOperatorId;

  useEffect(() => {
    if (previousDagOperatorId.current === dagOperatorId) {
      return;
    }
    previousDagOperatorId.current = dagOperatorId;
    setTableState(previous => ({
      ...previous,
      manualOperatorOverride: null,
      page: 0,
      selected: null,
      selectedEntityId: null,
    }));
  }, [dagOperatorId, setTableState]);

  const updateFilters = useCallback(
    (patch: Partial<EntityFilters>, options?: { preserveSelection?: boolean }) => {
      setTableState(previous => ({
        ...previous,
        filters: { ...(previous.filters ?? defaults), ...patch },
        page: 0,
        selected: options?.preserveSelection ? previous.selected : null,
        selectedEntityId: options?.preserveSelection ? previous.selectedEntityId : null,
      }));
    },
    [defaults, setTableState]
  );

  const updateSortDir = useCallback(
    (sortDir: SortDir) => {
      setTableState(previous => ({
        ...previous,
        filters: { ...(previous.filters ?? defaults), sortDir },
        page: 0,
      }));
    },
    [defaults, setTableState]
  );

  const updateOperator = useCallback(
    (value: string | null) => {
      setTableState(previous => ({
        ...previous,
        manualOperatorOverride: { dagOperatorId, value },
        page: 0,
        selected: null,
        selectedEntityId: null,
      }));
    },
    [dagOperatorId, setTableState]
  );

  const resetFilters = useCallback(() => {
    setTableState(previous => ({
      ...previous,
      filters: defaults,
      manualOperatorOverride: { dagOperatorId, value: null },
      page: 0,
      selected: null,
      selectedEntityId: null,
    }));
  }, [dagOperatorId, defaults, setTableState]);

  const setPage = useCallback(
    (value: SetStateAction<number>) => {
      setTableState(previous => ({
        ...previous,
        page: typeof value === 'function' ? value(previous.page) : value,
      }));
    },
    [setTableState]
  );

  const setSelected = useCallback(
    (value: SetStateAction<FiniteStateMachine | null>) => {
      setTableState(previous => {
        const selected = typeof value === 'function' ? value(previous.selected) : value;
        return {
          ...previous,
          selected,
          selectedEntityId: selected?.id ?? null,
        };
      });
    },
    [setTableState]
  );

  const operatorOptions = useMemo<SelectFieldOption[]>(
    () =>
      Object.values(entities.operators)
        .map(operator => ({
          value: operator.id,
          label: operator.instance_name ?? operator.operator_type_name ?? operator.id,
        }))
        .sort((a, b) => a.label.localeCompare(b.label)),
    [entities.operators]
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
        }))
        .sort((a, b) => a.label.localeCompare(b.label)),
    [entities.resources]
  );
  const resourceLabel = useCallback(
    (id: string) => {
      const resource = entities.resources[id];
      return resource ? `${resource.instance_name} (${resource.type_name})` : id;
    },
    [entities.resources]
  );
  const operatorLabel = useCallback(
    (id: string) => {
      const operator = entities.operators[id];
      return operator ? (operator.instance_name ?? operator.operator_type_name ?? id) : id;
    },
    [entities.operators]
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
    () => buildEntityRequest({ filters: effectiveFilters, operatorId, page, queryId, durationS }),
    [durationS, effectiveFilters, operatorId, page, queryId]
  );
  const query = useEntities({ engineId, request }, { enabled: validationErrors.length === 0 });
  const requestPending = query.isFetching;
  const rows = useMemo(() => entityRows(query.data), [query.data]);
  useEffect(() => {
    if (!selectedEntityId || selected?.id === selectedEntityId) {
      return;
    }
    const matchingEntity = rows.find(row => row.fsm.id === selectedEntityId)?.fsm;
    if (!matchingEntity) {
      return;
    }
    setTableState(previous =>
      previous.selectedEntityId === selectedEntityId
        ? { ...previous, selected: matchingEntity }
        : previous
    );
  }, [rows, selected?.id, selectedEntityId, setTableState]);
  const pageSize = normalizePageSize(filters.pageSize);
  const total = query.data?.total ?? 0;
  const pageCount = Math.max(1, Math.ceil(total / pageSize));
  const visibleStart = total === 0 ? 0 : page * pageSize + 1;
  const visibleEnd = total === 0 ? 0 : Math.min(total, visibleStart + rows.length - 1);
  const activeFilterCount = activeEntityFilterCount(filters, defaults, operatorId);

  return {
    filters: {
      values: filters,
      durationS,
      windowMaxS,
      validationErrors,
      invalidFilterFields,
      hasNonDefaultSettings: hasNonDefaultEntitySettings(filters, defaults, activeFilterCount),
      activeFilterCount,
      operatorId,
      operatorOptions,
      entityTypeOptions,
      resourceOptions,
      update: updateFilters,
      updateOperator,
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
