// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { createStore } from 'jotai';
import { showNestedChartRowsAtom } from '@quent/hooks';
import {
  defaultExpandedIdsAtom,
  expandedIdsAtom,
  pinnedCollapsedAtom,
  pinnedIdsAtom,
  pinnedPanelSizeAtom,
  resourceFilterAtom,
  rootResourceTypeAtom,
  selectedFsmTypesAtom,
  selectedNvtxCategoriesAtom,
  selectedNvtxDomainAtom,
  selectedTypesAtom,
} from '@/atoms/resourceTree';
import { isResourceFilterActive } from '@/features/resource-filter/resourceFilter';
import type { ViewStateV3 } from './viewState.schema';

type JotaiStore = ReturnType<typeof createStore>;
type ViewStateResources = NonNullable<ViewStateV3['resources']>;
type ResourcesPart = Partial<ViewStateResources>;

function sortedBy<T>(values: T[], key: (value: T) => string): T[] {
  return [...values].sort((a, b) => key(a).localeCompare(key(b)));
}

// Rows the app opens by itself are left out. Only the user's changes to them are shared.
function captureRows(store: JotaiStore): ResourcesPart {
  const expandedIds = store.get(expandedIdsAtom);
  const defaultExpandedIds = store.get(defaultExpandedIdsAtom);
  const expandedRowIds = [...expandedIds].filter(id => !defaultExpandedIds.has(id)).sort();
  const collapsedRowIds = [...defaultExpandedIds].filter(id => !expandedIds.has(id)).sort();
  return {
    ...(expandedRowIds.length > 0 ? { expandedRowIds } : {}),
    ...(collapsedRowIds.length > 0 ? { collapsedRowIds } : {}),
    ...(store.get(showNestedChartRowsAtom) ? { showNestedChartRows: true } : {}),
  };
}

function capturePinned(store: JotaiStore): ResourcesPart {
  // Keep the order the user arranged the pins in.
  const pinnedRowIds = [...store.get(pinnedIdsAtom)];
  const panelSize = store.get(pinnedPanelSizeAtom);
  return {
    ...(pinnedRowIds.length > 0 ? { pinnedRowIds } : {}),
    ...(store.get(pinnedCollapsedAtom) ? { pinnedRowsCollapsed: true as const } : {}),
    ...(panelSize !== undefined && panelSize > 0
      ? { pinnedRowsPanelSize: Math.round(panelSize) }
      : {}),
  };
}

function captureNvtx(store: JotaiStore): ResourcesPart {
  const domainId = store.get(selectedNvtxDomainAtom);
  const categories = sortedBy(
    [...store.get(selectedNvtxCategoriesAtom)].map(([domainId, categoryId]) => ({
      domainId,
      categoryId,
    })),
    category => category.domainId
  );
  if (!domainId && categories.length === 0) {
    return {};
  }
  return {
    nvtxSelection: {
      ...(domainId ? { domainId } : {}),
      ...(categories.length > 0 ? { categories } : {}),
    },
  };
}

function captureResourceFilter(store: JotaiStore): ResourcesPart {
  const filter = store.get(resourceFilterAtom);
  if (!isResourceFilterActive(filter)) {
    return {};
  }
  return {
    resourceFilter: {
      ...(filter.search.trim() ? { search: filter.search.trim() } : {}),
      ...(filter.resourceTypes.length > 0
        ? { resourceTypes: [...filter.resourceTypes].sort() }
        : {}),
      ...(filter.fsmTypes.length > 0 ? { fsmTypes: [...filter.fsmTypes].sort() } : {}),
      ...(filter.showOthers ? { showOthers: true } : {}),
    },
  };
}

function captureTypeSelections(
  store: JotaiStore,
  defaultRootResourceType: string | null
): ResourcesPart {
  const rootResourceType = store.get(rootResourceTypeAtom);
  const resourceTypeSelections = sortedBy(
    [...store.get(selectedTypesAtom)].map(([rowId, resourceType]) => ({ rowId, resourceType })),
    selection => selection.rowId
  );
  const fsmSelections = sortedBy(
    [...store.get(selectedFsmTypesAtom)].map(([rowId, fsmType]) => ({ rowId, fsmType })),
    selection => selection.rowId
  );
  return {
    ...(rootResourceType && rootResourceType !== defaultRootResourceType
      ? { rootResourceType }
      : {}),
    ...(resourceTypeSelections.length > 0 ? { resourceTypeSelections } : {}),
    ...(fsmSelections.length > 0 ? { fsmSelections } : {}),
  };
}

export function captureResources(
  store: JotaiStore,
  defaultRootResourceType: string | null
): ViewStateV3['resources'] {
  const resources: ViewStateResources = {
    ...captureRows(store),
    ...capturePinned(store),
    ...captureNvtx(store),
    ...captureResourceFilter(store),
    ...captureTypeSelections(store, defaultRootResourceType),
  };
  return Object.keys(resources).length > 0 ? resources : undefined;
}
