// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useMemo, useRef, type SetStateAction } from 'react';
import { useAtom } from 'jotai';
import { useEntities, useEntityList } from '@quent/client';
import {
  useOperatorSelection,
  useOperatorSelectionActions,
  useSelectedOperatorIds,
} from '@quent/hooks';
import type { OptionMultiSelectOption, SelectFieldOption } from '@quent/components';
import {
  resolveSelectedOperatorSelections,
  toggleOperatorSelection as resolveOperatorSelectionToggle,
  type EntityRef,
  type FiniteStateMachine,
  type QueryBundle,
  type SortDir,
} from '@quent/utils';
import { entitiesTableStateAtom } from '@/atoms/entitiesTable';
import { useDebouncedValue } from '@/hooks/useDebouncedValue';
import type { EntityFilters } from './types';
import {
  activeEntityFilterCount,
  buildEntityRequest,
  defaultEntityFilters,
  entityRows,
  hasNonDefaultEntitySettings,
  normalizePageSize,
  operatorLocationDescription,
  parseOptionalNumber,
  resourceLocationDescription,
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
  const operatorSelection = useOperatorSelection();
  const operatorIds = useSelectedOperatorIds();
  const updateOperatorSelection = useOperatorSelectionActions();
  const operators = useMemo(() => Object.values(entities.operators), [entities.operators]);
  const defaults = useMemo(() => defaultEntityFilters(durationS), [durationS]);
  const [tableState, setTableState] = useAtom(entitiesTableStateAtom);
  const filters = tableState.filters ?? defaults;
  const { page, selected, selectedEntityId } = tableState;
  // Bound minimum usage by the longest entity, not the often much longer query.
  const longestEntityQuery = useEntityList({
    engineId,
    queryId,
    window: { start: 0, end: durationS },
    sortKey: 'UsageDuration',
    sortDir: 'Desc',
    maxItems: 1,
  });
  const maxUsageS = longestEntityQuery.data?.items[0]?.usage_duration_s ?? durationS;
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
        return { ...previous, selected, selectedEntityId: selected?.id ?? null };
      });
    },
    [setTableState]
  );
  const filtersRef = useRef(filters);
  filtersRef.current = filters;
  // Clamp only when the fetched usage bound narrows.
  useEffect(() => {
    const currentMinUsageS = parseOptionalNumber(filtersRef.current.minUsageS);
    if (currentMinUsageS === null || currentMinUsageS <= maxUsageS) {
      return;
    }
    setTableState(previous => ({
      ...previous,
      filters: { ...(previous.filters ?? defaults), minUsageS: String(maxUsageS) },
      page: 0,
      selected: null,
      selectedEntityId: null,
    }));
  }, [defaults, maxUsageS, setTableState]);
  const operatorLabel = useCallback(
    (id: string) => {
      const operator = entities.operators[id];
      return operator ? (operator.instance_name ?? operator.operator_type_name ?? id) : id;
    },
    [entities.operators]
  );
  const operatorIdsKey = [...operatorIds].sort().join('\0');
  const previousOperatorIdsKey = useRef(operatorIdsKey);
  // Reset pagination/selection whenever the operator filter changes, regardless of whether
  // it came from this toolbar or another crossfiltered view (DAG, operator swimlanes, etc).
  useEffect(() => {
    if (previousOperatorIdsKey.current === operatorIdsKey) {
      return;
    }
    previousOperatorIdsKey.current = operatorIdsKey;
    setPage(0);
    setSelected(null);
  }, [operatorIdsKey, setPage, setSelected]);

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

  const applyOperatorSelection = useCallback(
    (nextIds: Set<string>) => {
      updateOperatorSelection({
        type: 'replace',
        selections: resolveSelectedOperatorSelections(operators, nextIds),
      });
      setPage(0);
      setSelected(null);
    },
    [operators, setPage, setSelected, updateOperatorSelection]
  );

  const toggleOperator = useCallback(
    (value: string) => {
      const nextSelections = resolveOperatorSelectionToggle(
        operators,
        operatorIds,
        operatorSelection.selections,
        value
      );
      updateOperatorSelection({
        type: 'replace',
        selections: resolveSelectedOperatorSelections(
          operators,
          nextSelections.flatMap(selection => [...selection.operatorIds])
        ),
      });
      setPage(0);
      setSelected(null);
    },
    [
      operatorIds,
      operatorSelection.selections,
      operators,
      setPage,
      setSelected,
      updateOperatorSelection,
    ]
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
    updateOperatorSelection({ type: 'clear' });
    setTableState(previous => ({
      ...previous,
      filters: defaults,
      page: 0,
      selected: null,
      selectedEntityId: null,
    }));
  }, [defaults, setTableState, updateOperatorSelection]);

  const operatorOptions = useMemo<OptionMultiSelectOption[]>(
    () =>
      operators
        .map(operator => ({
          value: operator.id,
          label: operator.instance_name ?? operator.operator_type_name ?? operator.id,
          description: operatorLocationDescription(operator, entities.plans, entities.workers),
        }))
        .sort((a, b) => (a.label ?? '').localeCompare(b.label ?? '')),
    [entities.plans, entities.workers, operators]
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
  const activeFilterCount = activeEntityFilterCount(filters, defaults, operatorIds);

  return {
    filters: {
      values: filters,
      durationS,
      maxUsageS,
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
