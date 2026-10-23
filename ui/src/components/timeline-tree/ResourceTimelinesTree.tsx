// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { useAtom } from 'jotai';
import { useCallback, useDeferredValue, useEffect, useMemo } from 'react';
import type { ReactNode } from 'react';
import { fetchSingleTimeline, DEFAULT_STALE_TIME } from '@quent/client';
import {
  DataText,
  ResourceColumn,
  UsageColumn,
  buildBulkParamsForItem,
  collectVisibleEntries,
  findItemById,
  getAdaptiveNumBins,
  transformResourceTree,
  type TreeTableItem,
} from '@quent/components';
import { useBulkTimelines, useHighlightedItemIds } from '@quent/hooks';
import {
  ALL_FSMS,
  type EntityRef,
  type OperatorFilter,
  type QueryBundle,
  type QueryFilter,
  type ResourceTree,
  type SingleTimelineRequest,
  unpackEntityRef,
  resolveResourceFsmType,
  resourceFsmChoices,
  workerDisplayName,
} from '@quent/utils';
import {
  resourceFilterAtom,
  rootResourceTypeAtom,
  selectedFsmTypesAtom,
  selectedTypesAtom,
} from '@/atoms/resourceTree';
import { resolvePinnedRows } from '@/features/pinned-rows/pinnedRows';
import { ResourceFilterSearch } from '@/features/resource-filter/ResourceFilterSearch';
import { filterResourceTree } from '@/features/resource-filter/resourceFilter';
import { useAutoExpandMatchingAncestors, useExpandedIds } from '@/hooks/useExpandedIds';
import { usePinnedCollapsed, usePinnedIds } from '@/hooks/usePinnedIds';
import type { ResourceTimelineSubRow } from './sub-rows';
import {
  TimelineTreeTable,
  useTimelineTreeSetup,
  type TimelineTreeControls,
  type TimelineTreeItem,
  type TimelineTreeModel,
  type PinningControls,
  type RowPinning,
} from './TimelineTreeTable';

function getRootResourceGroupId(resourceTree: ResourceTree<EntityRef>): string | null {
  if (!('ResourceGroup' in resourceTree)) {
    return null;
  }
  return unpackEntityRef(resourceTree.ResourceGroup.id).id;
}

export interface ResourceTimelinesTreeModel extends TimelineTreeModel, TimelineTreeControls {
  filterMatchCount: number;
  isFilterActive: boolean;
  rootItem: TreeTableItem;
  showOthers: boolean;
}

export type { ResourceTimelineSubRow };

const EMPTY_SUB_ROWS: readonly ResourceTimelineSubRow[] = [];

function getEntityName(item: TreeTableItem): string | undefined {
  return 'instance_name' in item.entity ? (item.entity.instance_name ?? undefined) : undefined;
}

export interface ResourceTimelinesTreeProps {
  engineId: string;
  queryBundle: QueryBundle<EntityRef>;
  subRows?: readonly ResourceTimelineSubRow[];
  seedRootExpanded?: boolean;
}

interface UseResourceTimelinesTreeModelProps extends ResourceTimelinesTreeProps {
  isDark: boolean;
}

// QueryResourceTree reuses the model to combine multiple trees in one table.

export function useResourceTimelinesTreeModel({
  engineId,
  queryBundle,
  isDark,
  subRows = EMPTY_SUB_ROWS,
  seedRootExpanded = true,
}: UseResourceTimelinesTreeModelProps): ResourceTimelinesTreeModel {
  const { entities, resource_tree: resourceTree } = queryBundle;
  const durationSeconds = queryBundle.duration_s;
  const [selectedTypes, setSelectedTypes] = useAtom(selectedTypesAtom);
  const [selectedFsmTypes, setSelectedFsmTypes] = useAtom(selectedFsmTypesAtom);
  const [rootResourceType, setRootResourceType] = useAtom(rootResourceTypeAtom);
  const [resourceFilter, setResourceFilter] = useAtom(resourceFilterAtom);
  const deferredResourceFilter = useDeferredValue(resourceFilter);

  const rootItem = useMemo(
    () => transformResourceTree(entities, resourceTree),
    [resourceTree, entities]
  );
  const indicatorItemIds = useHighlightedItemIds(rootItem);
  const resourceFilterResult = useMemo(
    () => filterResourceTree(rootItem, entities, deferredResourceFilter),
    [deferredResourceFilter, entities, rootItem]
  );
  useAutoExpandMatchingAncestors(
    rootItem,
    resourceFilterResult.directMatchIds,
    resourceFilterResult.isActive && resourceFilter.showOthers
  );
  const highlightedItemIds = useMemo(
    () =>
      resourceFilterResult.isActive && resourceFilter.showOthers
        ? resourceFilterResult.directMatchIds
        : new Set<string>(),
    [resourceFilter.showOthers, resourceFilterResult.directMatchIds, resourceFilterResult.isActive]
  );
  const resourceTypeOptions = useMemo(
    () => Object.keys(entities.resource_types).sort(),
    [entities.resource_types]
  );
  const fsmTypeOptions = useMemo(
    () =>
      [
        ...new Set(
          Object.keys(entities.fsm_types).concat(
            Object.values(entities.resource_types).flatMap(resourceType =>
              (resourceType?.display_order ?? []).flatMap(selection =>
                selection === ALL_FSMS ? [] : [selection.Type]
              )
            )
          )
        ),
      ].sort(),
    [entities.fsm_types, entities.resource_types]
  );

  useEffect(() => {
    if (rootResourceType != null) {
      return;
    }
    const initial = rootItem.availableResourceTypes?.[0] ?? resourceTypeOptions[0];
    if (initial) {
      setRootResourceType(initial);
    }
  }, [rootResourceType, resourceTypeOptions, rootItem.availableResourceTypes, setRootResourceType]);

  const injectSubRows = useCallback(
    (item: TreeTableItem) => subRows.reduce((current, subRow) => subRow.injectRows(current), item),
    [subRows]
  );
  // The whole tree with sub-rows, before any filter. Pinned rows are looked up here.
  const fullTree = useMemo(() => injectSubRows(rootItem), [injectSubRows, rootItem]);

  const trees = useMemo(() => {
    if (!resourceFilterResult.isActive || resourceFilter.showOthers) {
      return [fullTree];
    }
    if (resourceFilterResult.filteredItems.length === 0) {
      return [];
    }

    const filteredRoot = { ...rootItem, children: resourceFilterResult.filteredItems };
    return injectSubRows(filteredRoot).children ?? [];
  }, [
    fullTree,
    injectSubRows,
    resourceFilter.showOthers,
    resourceFilterResult.filteredItems,
    resourceFilterResult.isActive,
    rootItem,
  ]);

  const { pinnedIds, togglePin, movePin, isAtPinLimit } = usePinnedIds();
  const { isPinnedCollapsed, togglePinnedCollapsed } = usePinnedCollapsed();
  // A pin whose row a filter has removed is hidden, not forgotten; clearing the filter restores it.
  const { shownItems: pinnedItems, hiddenCount: hiddenPinnedCount } = useMemo(
    () => resolvePinnedRows(pinnedIds, [fullTree], trees),
    [fullTree, pinnedIds, trees]
  );
  const getPinnedRowDescription = useCallback(
    (item: TimelineTreeItem) => {
      const findWorkerName = (
        row: TimelineTreeItem,
        workerAbove?: string
      ): string | null | undefined => {
        if (row.id === item.id) {
          return workerAbove;
        }
        const worker = entities.workers[row.id];
        const workerAtOrAbove = worker ? workerDisplayName(worker) : workerAbove;
        for (const child of row.children ?? []) {
          const name = findWorkerName(child, workerAtOrAbove);
          if (name !== null) {
            return name;
          }
        }
        return null;
      };

      const workerName = findWorkerName(fullTree as TimelineTreeItem);
      return workerName ? (
        <>
          Worker: <DataText className="text-foreground">{workerName}</DataText>
        </>
      ) : undefined;
    },
    [entities.workers, fullTree]
  );
  // The root group is already summarized by the overview bar, so it has no pin button.
  const isPinnable = useCallback(
    (item: TimelineTreeItem) => item.id !== rootItem.id,
    [rootItem.id]
  );
  const getRowName = useCallback(
    (item: TimelineTreeItem) =>
      subRows.find(subRow => subRow.matches(item))?.label ??
      getEntityName(item as TreeTableItem) ??
      item.id,
    [subRows]
  );
  const rowPinning = useMemo<RowPinning>(
    () => ({ isPinnable, getName: getRowName, getDescription: getPinnedRowDescription }),
    [getPinnedRowDescription, getRowName, isPinnable]
  );
  const pinning = useMemo<PinningControls>(
    () => ({
      items: pinnedItems as TimelineTreeItem[],
      hiddenCount: hiddenPinnedCount,
      ids: pinnedIds,
      isAtLimit: isAtPinLimit,
      onToggle: togglePin,
      isCollapsed: isPinnedCollapsed,
      onToggleCollapsed: togglePinnedCollapsed,
      onReorder: movePin,
    }),
    [
      hiddenPinnedCount,
      isAtPinLimit,
      isPinnedCollapsed,
      movePin,
      pinnedIds,
      pinnedItems,
      togglePin,
      togglePinnedCollapsed,
    ]
  );
  // Only real resources and groups have bulk timeline data; sub-rows load their own.
  const pinnedBulkItems = useMemo(
    () =>
      pinnedItems.flatMap(item => {
        const resourceItem = findItemById(rootItem, item.id);
        return resourceItem ? [resourceItem] : [];
      }),
    [pinnedItems, rootItem]
  );
  const firstResourceTimelineId = trees[0]?.id ?? null;

  const { expandedIds, handleExpandChange } = useExpandedIds(
    seedRootExpanded ? rootItem.id : undefined
  );
  const bulkRootItem = useMemo(
    () =>
      !resourceFilterResult.isActive || resourceFilter.showOthers
        ? rootItem
        : resourceFilterResult.filteredItems.length === 1
          ? resourceFilterResult.filteredItems[0]!
          : { ...rootItem, children: resourceFilterResult.filteredItems },
    [
      resourceFilter.showOthers,
      resourceFilterResult.filteredItems,
      resourceFilterResult.isActive,
      rootItem,
    ]
  );
  const { handleZoomChange, handleExpand } = useBulkTimelines({
    engineId,
    queryId: queryBundle.query_id,
    rootItem: bulkRootItem,
    expandedIds,
    selectedTypes,
    groupFsmFilters: selectedFsmTypes,
    entities,
    collectVisibleEntriesFn: collectVisibleEntries,
    buildBulkParamsFn: buildBulkParamsForItem,
    findItemByIdFn: findItemById,
    extraItems: pinnedBulkItems,
  });

  const onExpandChange = useCallback(
    (itemId: string, isExpanded: boolean) => {
      handleExpandChange(itemId, isExpanded);
      handleExpand(itemId, isExpanded);
    },
    [handleExpandChange, handleExpand]
  );

  const rootResourceGroupId = useMemo(() => getRootResourceGroupId(resourceTree), [resourceTree]);
  const { data: fetchedRootTimeline } = useQuery({
    queryKey: [
      'resourceGroupTimeline',
      'root',
      engineId,
      queryBundle.query_id,
      rootResourceGroupId,
      durationSeconds,
      rootResourceType,
    ],
    queryFn: () => {
      const request: SingleTimelineRequest<QueryFilter, OperatorFilter> = {
        entry: {
          ResourceGroup: {
            resource_group_id: rootResourceGroupId!,
            resource_type_name: rootResourceType ?? '',
            long_entities_threshold_s: null,
            entity_filter: { entity_type_name: null },
            app_params: { operator_ids: [] },
            config: {
              num_bins: getAdaptiveNumBins(),
              start: 0,
              end: durationSeconds,
            },
          },
        },
        app_params: { query_id: queryBundle.query_id },
      };
      return fetchSingleTimeline(engineId, request, durationSeconds);
    },
    staleTime: DEFAULT_STALE_TIME,
    enabled: rootResourceGroupId != null && !!rootResourceType,
    placeholderData: keepPreviousData,
  });

  const renderLabel = useCallback(
    (item: TimelineTreeItem, description?: ReactNode, isPinned?: boolean) => {
      const subRow = subRows.find(candidate => candidate.matches(item));
      if (subRow) {
        return subRow.renderLabel(item, description);
      }

      const selectedType = selectedTypes.get(item.id) || item.availableResourceTypes?.[0] || '';
      const groupDeclaration =
        entities.resource_group_types[
          item.entity && 'type_name' in item.entity ? item.entity.type_name : item.type
        ];
      const availableFsmTypes = resourceFsmChoices(
        entities.resource_types[selectedType],
        groupDeclaration
      );
      return (
        <ResourceColumn
          item={item as TreeTableItem}
          description={description}
          className={isPinned ? undefined : 'cursor-pointer'}
          selectedType={selectedType}
          onTypeChange={(itemId, newType) => {
            setSelectedTypes(previous => new Map(previous).set(itemId, newType));
            if (itemId === rootItem.id) {
              setRootResourceType(newType);
            }
          }}
          availableFsmTypes={availableFsmTypes}
          selectedFsmType={resolveResourceFsmType({
            resourceType: entities.resource_types[selectedType],
            selectedFsmType: selectedFsmTypes.get(item.id),
            resourceGroupType: groupDeclaration,
          })}
          onFsmChange={(itemId, fsmType) => {
            setSelectedFsmTypes(previous => new Map(previous).set(itemId, fsmType));
          }}
        />
      );
    },
    [
      entities.resource_types,
      entities.resource_group_types,
      rootItem.id,
      selectedFsmTypes,
      selectedTypes,
      setRootResourceType,
      setSelectedFsmTypes,
      setSelectedTypes,
      subRows,
    ]
  );

  const renderTimeline = useCallback(
    (item: TimelineTreeItem) => {
      const subRow = subRows.find(candidate => candidate.matches(item));
      if (subRow) {
        return subRow.renderTimeline(item);
      }

      return (
        <UsageColumn
          item={item as TreeTableItem}
          engineId={engineId}
          queryBundle={queryBundle}
          selectedTypes={selectedTypes}
          selectedFsmTypes={selectedFsmTypes}
          durationSeconds={durationSeconds}
          isDark={isDark}
          showPlayheadIndicator={item.id === firstResourceTimelineId}
        />
      );
    },
    [
      durationSeconds,
      engineId,
      firstResourceTimelineId,
      isDark,
      queryBundle,
      selectedFsmTypes,
      selectedTypes,
      subRows,
    ]
  );

  return {
    filterMatchCount: resourceFilterResult.matchCount,
    isFilterActive: resourceFilterResult.isActive,
    rootItem,
    showOthers: resourceFilter.showOthers,
    trees: trees as TimelineTreeItem[],
    pinning,
    rowPinning,
    emptyMessage: 'No resources match this filter.',
    filters: (
      <ResourceFilterSearch
        fsmTypes={fsmTypeOptions}
        matchCount={resourceFilterResult.matchCount}
        onFsmTypesChange={fsmTypes => setResourceFilter(current => ({ ...current, fsmTypes }))}
        onResourceTypesChange={resourceTypes =>
          setResourceFilter(current => ({ ...current, resourceTypes }))
        }
        onSearchChange={search => setResourceFilter(current => ({ ...current, search }))}
        onShowOthersChange={showOthers =>
          setResourceFilter(current => ({ ...current, showOthers }))
        }
        resourceTypes={resourceTypeOptions}
        search={resourceFilter.search}
        selectedFsmTypes={resourceFilter.fsmTypes}
        selectedResourceTypes={resourceFilter.resourceTypes}
        showOthers={resourceFilter.showOthers}
      />
    ),
    initialSelectedItemId: rootItem.id,
    expandedIds,
    highlightedItemIds,
    indicatorItemIds,
    timelineData: fetchedRootTimeline,
    onExpandChange,
    onZoomChange: handleZoomChange,
    renderLabel,
    renderTimeline,
  };
}

export function ResourceTimelinesTree({
  engineId,
  queryBundle,
  subRows,
  seedRootExpanded,
}: ResourceTimelinesTreeProps) {
  const { durationSeconds, isDark } = useTimelineTreeSetup(queryBundle);
  const resourceTree = useResourceTimelinesTreeModel({
    engineId,
    queryBundle,
    isDark,
    subRows,
    seedRootExpanded,
  });
  const trees =
    resourceTree.isFilterActive && resourceTree.showOthers && resourceTree.filterMatchCount === 0
      ? []
      : [resourceTree];

  return (
    <TimelineTreeTable
      durationSeconds={durationSeconds}
      isDark={isDark}
      trees={trees}
      controls={resourceTree}
    />
  );
}
