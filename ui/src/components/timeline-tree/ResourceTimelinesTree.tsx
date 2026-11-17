// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { useAtom } from 'jotai';
import { useCallback, useDeferredValue, useEffect, useMemo } from 'react';
import type { ReactNode } from 'react';
import { fetchSingleTimeline, DEFAULT_STALE_TIME } from '@quent/client';
import {
  DEFAULT_TIMELINE_HEIGHT,
  ResourceColumn,
  UsageColumn,
  buildBulkParamsForItem,
  collectVisibleEntries,
  findItemById,
  getAdaptiveNumBins,
  transformResourceTree,
  type TreeTableItem,
} from '@quent/components';
import { useBulkTimelines, useHighlightedItemIds, useShowNestedChartRows } from '@quent/hooks';
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
import { findAncestorNames, resolvePinnedRows } from '@/features/pinned-rows/pinnedRows';
import { filterResourceTree } from '@/features/resource-filter/resourceFilter';
import {
  useAutoExpandMatchingAncestors,
  useExpandedIds,
  useInitializeDisclosureExpansion,
} from '@/hooks/useExpandedIds';
import { usePinnedIds } from '@/hooks/usePinnedIds';
import { PinnedRowPath } from './PinnedRowPath';
import { TIMELINE_SUB_ROW_DISCLOSURE_HEIGHT, type ResourceTimelineSubRow } from './sub-rows';
import { TimelineTreeFilterControls } from './TimelineTreeFilterControls';
import {
  TimelineTreeTable,
  useTimelineTreeSetup,
  type TimelineTreeControls,
  type TimelineTreeItem,
  type TimelineTreeModel,
  type RowPinning,
} from './TimelineTreeTable';
import type {
  TimelineTreeFilterSummary,
  TimelineTreePinningSource,
} from './timelineTreeComposition';
import { useCombinedTimelineTreePinning } from './timelineTreeComposition';

function getRootResourceGroupId(resourceTree: ResourceTree<EntityRef>): string | null {
  if (!('ResourceGroup' in resourceTree)) {
    return null;
  }
  return unpackEntityRef(resourceTree.ResourceGroup.id).id;
}

export interface ResourceTimelinesTreeModel
  extends TimelineTreeModel, Omit<TimelineTreeControls, 'filters' | 'pinning'> {
  filter: TimelineTreeFilterSummary;
  filterOptions: {
    fsmTypes: string[];
    resourceTypes: string[];
  };
  pinningSource: TimelineTreePinningSource;
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
  const [resourceFilter] = useAtom(resourceFilterAtom);
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
  const showNestedChartRows = useShowNestedChartRows();
  const disclosureExpansion = useMemo(() => {
    const subRowsByDisclosureType = new Map(
      subRows.map(subRow => [subRow.disclosureRowType, subRow])
    );
    const expansion: { id: string; ownerId?: string }[] = [];
    const visit = (item: TimelineTreeItem) => {
      for (const child of item.children ?? []) {
        const subRow = subRowsByDisclosureType.get(child.type);
        if (subRow) {
          expansion.push({
            id: child.id,
            ...(subRow.expandOwnerByDefault ? { ownerId: item.id } : {}),
          });
        }
        visit(child);
      }
    };
    visit(fullTree as TimelineTreeItem);
    return expansion;
  }, [fullTree, subRows]);
  useInitializeDisclosureExpansion(disclosureExpansion, showNestedChartRows);

  const trees = useMemo(() => {
    if (!resourceFilterResult.isActive) {
      return [fullTree];
    }
    if (resourceFilter.showOthers) {
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

  const { pinnedIds } = usePinnedIds();
  // The root and sub-rows owned by another row cannot be pinned independently.
  const isPinnable = useCallback(
    (item: TimelineTreeItem) =>
      item.id !== rootItem.id &&
      !subRows.some(
        subRow =>
          subRow.disclosureRowType === item.type || (subRow.pinsWithOwner && subRow.matches(item))
      ),
    [rootItem.id, subRows]
  );
  const preparePinnedItem = useCallback(
    (item: TimelineTreeItem) => {
      const ownedRows = (item.children ?? []).filter(child =>
        subRows.some(subRow => subRow.pinsWithOwner && subRow.disclosureRowType === child.type)
      );
      return { ...item, children: ownedRows.length > 0 ? ownedRows : undefined };
    },
    [subRows]
  );
  // A pin whose row a filter has removed is hidden, not forgotten; clearing the filter restores it.
  const { shownItems: pinnedItems } = useMemo(() => {
    const resolved = resolvePinnedRows(pinnedIds, [fullTree], trees, isPinnable);
    return { shownItems: resolved.shownItems.map(preparePinnedItem) };
  }, [fullTree, isPinnable, pinnedIds, preparePinnedItem, trees]);
  const getPinnedRowDescription = useCallback(
    (item: TimelineTreeItem) => {
      // A row pinned with its owner (Entities) is drawn under that owner, which already has the path.
      if (subRows.some(subRow => subRow.pinsWithOwner && subRow.matches(item))) {
        return undefined;
      }
      const getName = (row: TimelineTreeItem) => {
        const worker = entities.workers[row.id];
        return worker ? workerDisplayName(worker) : (getEntityName(row as TreeTableItem) ?? row.id);
      };
      const roots = [fullTree as TimelineTreeItem];
      const names = findAncestorNames(roots, item.id, getName);
      return names?.length ? <PinnedRowPath names={names} /> : undefined;
    },
    [entities.workers, fullTree, subRows]
  );
  // Entities and Operators rows are left out of "pin all" for now, and so is the root.
  const pinnableMatchIds = useMemo(() => {
    const { directMatchIds } = resourceFilterResult;
    const ids: string[] = [];
    const visit = (item: TimelineTreeItem) => {
      const isSubRow = subRows.some(subRow => subRow.matches(item));
      if (!isSubRow && directMatchIds.has(item.id) && item.id !== rootItem.id) {
        ids.push(item.id);
      }
      item.children?.forEach(visit);
    };
    (trees as TimelineTreeItem[]).forEach(visit);
    return ids;
  }, [resourceFilterResult, rootItem.id, subRows, trees]);
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
        return subRow.renderTimeline(item, expandedIds.has(item.id));
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
      expandedIds,
    ]
  );
  // A disclosure row is a thin header while closed. While open it draws the chart, so it is as
  // tall as one.
  const getRowHeight = useCallback(
    (item: TimelineTreeItem) =>
      subRows.some(subRow => subRow.disclosureRowType === item.type) && !expandedIds.has(item.id)
        ? TIMELINE_SUB_ROW_DISCLOSURE_HEIGHT
        : DEFAULT_TIMELINE_HEIGHT,
    [expandedIds, subRows]
  );
  const pinningSource = useMemo<TimelineTreePinningSource>(
    () => ({
      fullRoots: [fullTree],
      displayedRoots: trees as TimelineTreeItem[],
      isPinnable,
      preparePinnedItem,
    }),
    [fullTree, isPinnable, preparePinnedItem, trees]
  );

  return {
    filter: {
      isActive: resourceFilterResult.isActive,
      matchCount: resourceFilterResult.matchCount,
      pinnableMatchIds,
    },
    filterOptions: {
      fsmTypes: fsmTypeOptions,
      resourceTypes: resourceTypeOptions,
    },
    pinningSource,
    rootItem,
    showOthers: resourceFilter.showOthers,
    trees: trees as TimelineTreeItem[],
    rowPinning,
    getRowHeight,
    emptyMessage: 'No resources match this filter.',
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
  const pinningSources = useMemo(() => [resourceTree.pinningSource], [resourceTree.pinningSource]);
  const { pinning, pinMany } = useCombinedTimelineTreePinning(pinningSources);
  const filters = (
    <TimelineTreeFilterControls
      {...resourceTree.filterOptions}
      matchCount={resourceTree.filter.matchCount}
      matchIds={resourceTree.filter.pinnableMatchIds}
      onPinMany={pinMany}
      pinnedIds={pinning.ids}
    />
  );
  const trees =
    resourceTree.filter.isActive && resourceTree.showOthers && resourceTree.filter.matchCount === 0
      ? []
      : [resourceTree];

  return (
    <TimelineTreeTable
      durationSeconds={durationSeconds}
      isDark={isDark}
      trees={trees}
      controls={{ ...resourceTree, filters, pinning }}
    />
  );
}
