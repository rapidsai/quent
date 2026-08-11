// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useMemo, useState } from 'react';
import { useEntities } from '@quent/client';
import { useSelectedNodeIds } from '@quent/hooks';
import type { SelectFieldOption } from '@quent/components';
import type { EntityRef, FiniteStateMachine, QueryBundle } from '@quent/utils';
import { useDebouncedValue } from '@/hooks/useDebouncedValue';
import type { EntityFilters } from './types';
import {
  activeEntityFilterCount,
  buildEntityRequest,
  defaultEntityFilters,
  entityRows,
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

interface ManualOperatorOverride {
  dagOperatorId: string | null;
  value: string | null;
}

export function useEntityTable({ engineId, queryId, queryBundle }: UseEntityTableParams) {
  const { entities, duration_s: durationS } = queryBundle;
  const selectedNodeIds = useSelectedNodeIds();
  const dagOperatorId = selectedNodeIds.values().next().value ?? null;
  const defaults = useMemo(() => defaultEntityFilters(durationS), [durationS]);
  const [filters, setFilters] = useState<EntityFilters>(() => defaultEntityFilters(durationS));
  const [manualOperatorOverride, setManualOperatorOverride] =
    useState<ManualOperatorOverride | null>(null);
  const [page, setPage] = useState(0);
  const [selected, setSelected] = useState<FiniteStateMachine | null>(null);
  const operatorId =
    manualOperatorOverride?.dagOperatorId === dagOperatorId
      ? manualOperatorOverride.value
      : dagOperatorId;

  useEffect(() => {
    setManualOperatorOverride(null);
    setPage(0);
    setSelected(null);
  }, [dagOperatorId]);

  const updateFilters = useCallback(
    (patch: Partial<EntityFilters>, options?: { preserveSelection?: boolean }) => {
      setFilters(previous => ({ ...previous, ...patch }));
      setPage(0);
      if (!options?.preserveSelection) setSelected(null);
    },
    []
  );

  const updateOperator = useCallback(
    (value: string | null) => {
      setManualOperatorOverride({ dagOperatorId, value });
      setPage(0);
      setSelected(null);
    },
    [dagOperatorId]
  );

  const resetFilters = useCallback(() => {
    setFilters(defaults);
    setManualOperatorOverride({ dagOperatorId, value: null });
    setPage(0);
    setSelected(null);
  }, [dagOperatorId, defaults]);

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

  const validationErrors = useMemo(() => validateEntityFilters(filters), [filters]);

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
  const isDebouncing =
    filters.minUsageS !== debouncedMinUsageS ||
    filters.windowStart !== debouncedWindowStart ||
    filters.windowEnd !== debouncedWindowEnd;
  const requestPending = query.isFetching || isDebouncing;
  const rows = useMemo(() => entityRows(query.data), [query.data]);
  const pageSize = normalizePageSize(filters.pageSize);
  const total = query.data?.total ?? 0;
  const pageCount = Math.max(1, Math.ceil(total / pageSize));
  const visibleStart = total === 0 ? 0 : page * pageSize + 1;
  const visibleEnd = total === 0 ? 0 : Math.min(total, visibleStart + rows.length - 1);
  const activeFilterCount = activeEntityFilterCount(filters, defaults, operatorId);

  return {
    activeFilterCount,
    entityTypeOptions,
    fsmTypes: entities.fsm_types,
    error: query.error,
    filters,
    hasNonDefaultSettings: hasNonDefaultEntitySettings(filters, defaults, activeFilterCount),
    isError: query.isError,
    isLoading: query.isLoading,
    operatorId,
    operatorOptions,
    page,
    pageCount,
    paginationDisabled: requestPending || validationErrors.length > 0,
    requestPending,
    resetFilters,
    operatorLabel,
    resourceLabel,
    resourceOptions,
    rows,
    selected,
    setPage,
    setSelected,
    total,
    updateFilters,
    updateOperator,
    validationErrors,
    visibleEnd,
    visibleStart,
  };
}
