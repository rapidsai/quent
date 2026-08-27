// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { useAtom } from 'jotai';
import { useCallback, useEffect, useMemo } from 'react';
import { fetchSingleTimeline, DEFAULT_STALE_TIME } from '@quent/client';
import {
  ResourceColumn,
  UsageColumn,
  buildBulkParamsForItem,
  collectResourceTypesFromTree,
  collectVisibleEntries,
  findItemById,
  getAdaptiveNumBins,
  transformResourceTree,
  type TreeTableItem,
} from '@quent/components';
import { useBulkTimelines, useHighlightedItemIds } from '@quent/hooks';
import {
  type EntityRef,
  type EntityRefKey,
  type OperatorFilter,
  type QueryBundle,
  type QueryFilter,
  type ResourceTree,
  type SingleTimelineRequest,
} from '@quent/utils';
import {
  rootResourceTypeAtom,
  selectedFsmTypesAtom,
  selectedTypesAtom,
} from '@/atoms/resourceTree';
import { useExpandedIds } from '@/hooks/useExpandedIds';
import type { ResourceTimelineSubRow } from './sub-rows';
import {
  TimelineTreeTable,
  useTimelineTreeSetup,
  type TimelineTreeControls,
  type TimelineTreeItem,
  type TimelineTreeModel,
} from './TimelineTreeTable';

function getRootResourceGroupId(resourceTree: ResourceTree<EntityRef>): string | null {
  if (!('ResourceGroup' in resourceTree)) {
    return null;
  }
  const [, entityId] = Object.entries(resourceTree.ResourceGroup.id)[0] as [EntityRefKey, string];
  return entityId;
}

export interface ResourceTimelinesTreeModel extends TimelineTreeModel, TimelineTreeControls {
  rootItem: TreeTableItem;
}

export type { ResourceTimelineSubRow };

const EMPTY_SUB_ROWS: readonly ResourceTimelineSubRow[] = [];

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
// eslint-disable-next-line react-refresh/only-export-components
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

  const rootItem = useMemo(
    () => transformResourceTree(entities, resourceTree),
    [resourceTree, entities]
  );
  const highlightedItemIds = useHighlightedItemIds(rootItem);
  const resourceTypeOptions = useMemo(() => collectResourceTypesFromTree([rootItem]), [rootItem]);

  useEffect(() => {
    if (rootResourceType != null) {
      return;
    }
    const initial = resourceTypeOptions[0];
    if (initial) {
      setRootResourceType(initial);
    }
  }, [rootResourceType, resourceTypeOptions, setRootResourceType]);

  const { expandedIds, handleExpandChange } = useExpandedIds(
    seedRootExpanded ? rootItem.id : undefined
  );
  const { handleZoomChange, handleExpand } = useBulkTimelines({
    engineId,
    queryId: queryBundle.query_id,
    rootItem,
    expandedIds,
    selectedTypes,
    groupFsmFilters: selectedFsmTypes,
    entities,
    collectVisibleEntriesFn: collectVisibleEntries,
    buildBulkParamsFn: buildBulkParamsForItem,
    findItemByIdFn: findItemById,
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

  const tree = useMemo(
    () => subRows.reduce((item, subRow) => subRow.injectRows(item), rootItem),
    [rootItem, subRows]
  );

  const renderLabel = useCallback(
    (item: TimelineTreeItem) => {
      const subRow = subRows.find(candidate => candidate.matches(item));
      if (subRow) {
        return subRow.renderLabel(item);
      }

      const selectedType = selectedTypes.get(item.id) || item.availableResourceTypes?.[0] || '';
      const availableFsmTypes = selectedType
        ? entities.resource_types[selectedType]?.used_by
        : undefined;
      return (
        <ResourceColumn
          item={item as TreeTableItem}
          selectedType={selectedType}
          onTypeChange={(itemId, newType) => {
            setSelectedTypes(previous => new Map(previous).set(itemId, newType));
            if (itemId === rootItem.id) {
              setRootResourceType(newType);
            }
          }}
          availableFsmTypes={availableFsmTypes}
          selectedFsmType={selectedFsmTypes.get(item.id) ?? null}
          onFsmChange={(itemId, fsmType) => {
            setSelectedFsmTypes(previous => new Map(previous).set(itemId, fsmType));
          }}
        />
      );
    },
    [
      entities.resource_types,
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
        />
      );
    },
    [durationSeconds, engineId, isDark, queryBundle, selectedFsmTypes, selectedTypes, subRows]
  );

  return {
    rootItem,
    tree: tree as TimelineTreeItem,
    initialSelectedItemId: rootItem.id,
    expandedIds,
    highlightedItemIds,
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

  return (
    <TimelineTreeTable
      durationSeconds={durationSeconds}
      isDark={isDark}
      trees={[resourceTree]}
      controls={resourceTree}
    />
  );
}
