// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { createStore } from 'jotai';
import {
  LONG_ENTITY_DENSITY_DEFAULT,
  showNestedChartRowsAtom,
  type HydratableViewState,
} from '@quent/hooks';
import type { EntitiesTableState, EntityFilters } from '@/atoms/entitiesTable';
import {
  pinnedCollapsedAtom,
  pinnedIdsAtom,
  pinnedPanelSizeAtom,
  resourceFilterAtom,
  rootResourceTypeAtom,
  selectedFsmTypesAtom,
  selectedNvtxCategoriesAtom,
  selectedNvtxDomainAtom,
  selectedTypesAtom,
  sharedCollapsedIdsAtom,
} from '@/atoms/resourceTree';
import { EMPTY_RESOURCE_FILTER } from '@/features/resource-filter/resourceFilter';
import {
  DEFAULT_PAGE_SIZE,
  SORT_DESC,
  defaultEntityFilters,
} from '@/components/entities-table/utils';
import type { ViewStateFields } from './viewState.fields';
import type { ViewStateEntitiesState } from './viewState.schema';

type JotaiStore = ReturnType<typeof createStore>;

export function hydrateEntitiesState(
  state: ViewStateEntitiesState,
  durationSeconds: number
): EntitiesTableState {
  const filters: EntityFilters = {
    ...defaultEntityFilters(durationSeconds),
    entityType: state.entityType ?? null,
    resourceId: state.resourceId ?? null,
    minUsageS: state.minUsageS === undefined ? '' : String(state.minUsageS),
    windowStart: state.window ? String(state.window.start) : '0',
    windowEnd: state.window ? String(state.window.end) : String(durationSeconds),
    sortDir: state.sortDir ?? SORT_DESC,
    pageSize: state.pageSize ?? DEFAULT_PAGE_SIZE,
  };

  return {
    filters,
    page: state.page ?? 0,
    selected: null,
    selectedEntityId: state.selectedEntityId ?? null,
  };
}

function toResourceFilter(filter: NonNullable<ViewStateFields['resources']>['resourceFilter']) {
  return filter
    ? {
        fsmTypes: filter.fsmTypes ?? [],
        resourceTypes: filter.resourceTypes ?? [],
        search: filter.search ?? '',
        showOthers: filter.showOthers ?? false,
      }
    : EMPTY_RESOURCE_FILTER;
}

/**
 * Restored state describes the whole view, so anything it omits resets the session's value
 * (pins, nested rows, filters, collapsed rows) instead of keeping it.
 */
export function hydrateResources(store: JotaiStore, resources: ViewStateFields['resources']): void {
  store.set(pinnedIdsAtom, new Set(resources?.pinnedRowIds ?? []));
  store.set(resourceFilterAtom, toResourceFilter(resources?.resourceFilter));
  store.set(showNestedChartRowsAtom, resources?.showNestedChartRows ?? false);
  store.set(sharedCollapsedIdsAtom, new Set(resources?.collapsedRowIds ?? []));
  if (resources?.rootResourceType !== undefined) {
    store.set(rootResourceTypeAtom, resources.rootResourceType);
  }
  if (resources?.resourceTypeSelections !== undefined) {
    store.set(
      selectedTypesAtom,
      new Map(resources.resourceTypeSelections.map(entry => [entry.rowId, entry.resourceType]))
    );
  }
  if (resources?.fsmSelections !== undefined) {
    store.set(
      selectedFsmTypesAtom,
      new Map(resources.fsmSelections.map(entry => [entry.rowId, entry.fsmType]))
    );
  }
  store.set(pinnedCollapsedAtom, resources?.pinnedRowsCollapsed ?? false);
  store.set(pinnedPanelSizeAtom, resources?.pinnedRowsPanelSize);
  store.set(selectedNvtxDomainAtom, resources?.nvtxSelection?.domainId ?? null);
  store.set(
    selectedNvtxCategoriesAtom,
    new Map(
      resources?.nvtxSelection?.categories?.map(({ domainId, categoryId }) => [
        domainId,
        categoryId,
      ]) ?? []
    )
  );
}

/** The part of the restored state that `useSerializableViewState` hydrates. */
export function toHydratableViewState(fields: ViewStateFields): HydratableViewState {
  const { dataFlow } = fields;
  return {
    selection: fields.selection,
    dag: fields.dag,
    dataFlow: dataFlow
      ? {
          enabled: dataFlow.enabled,
          measure: dataFlow.measure,
          labelMeasure: dataFlow.labelMeasure,
          dimensions: dataFlow.dimensions,
          playheadS: dataFlow.playheadS,
        }
      : undefined,
    operatorTable: fields.operatorTable,
    timeline: {
      longEntityDensity: fields.timeline?.longEntityDensity ?? LONG_ENTITY_DENSITY_DEFAULT,
    },
  };
}
