// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Column, TreeTable } from '@/components/ui/tree-table';
import { useCallback, useMemo, useState } from 'react';
import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { useHighlightedItemIds, useBulkTimelines, useHydrateTimelineAtoms } from '@quent/hooks';
import { ResourceTree, QueryBundle } from '@quent/utils';
import type { EntityRef, SingleTimelineRequest, QueryFilter, TaskFilter } from '@quent/utils';
import { TimelineController } from './timeline/TimelineController';
import { collectResourceTypesFromTree } from '@/lib/resource.utils';
import { EntityRefKey } from '@/types';
import { TreeTableItem } from './resource-tree/types';
import { ResourceColumn } from './resource-tree/ResourceColumn';
import { UsageColumn } from './resource-tree/UsageColumn';
import { fetchSingleTimeline, DEFAULT_STALE_TIME } from '@quent/client';
import {
  transformResourceTree,
  getAdaptiveNumBins,
  nanosToMs,
  collectVisibleEntries,
  buildBulkParamsForItem,
  findItemById,
} from '@/lib/timeline.utils';
import { useExpandedIds } from '@/hooks/useExpandedIds';
import { TimelineToolbar } from './timeline/TimelineToolbar';

function getRootResourceGroupId(resourceTree: ResourceTree<EntityRef>): string | null {
  if (!('ResourceGroup' in resourceTree)) return null;
  const [, entityId] = Object.entries(resourceTree.ResourceGroup.id)[0] as [EntityRefKey, string];
  return entityId;
}

interface QueryResourceTreeProps {
  engineId: string;
  queryBundle: QueryBundle<EntityRef>;
}

export function QueryResourceTree(props: QueryResourceTreeProps) {
  return <QueryResourceTreeContent {...props} />;
}

function QueryResourceTreeContent({ queryBundle, engineId }: QueryResourceTreeProps) {
  const { entities, resource_tree: resourceTree } = queryBundle;
  const [selectedTypes, setSelectedTypes] = useState<Map<string, string>>(new Map());
  const [selectedFsmTypes, setSelectedFsmTypes] = useState<Map<string, string | null>>(new Map());

  const startTime = queryBundle.start_time_unix_ns;
  const durationSeconds = queryBundle.duration_s;
  const startTimeMs = useMemo(() => nanosToMs(startTime), [startTime]);

  useHydrateTimelineAtoms({
    zoomRange: { start: 0, end: durationSeconds },
    debouncedZoomRange: { start: 0, end: durationSeconds },
    startTimeMs,
  });

  const rootItem = useMemo(
    () => transformResourceTree(entities, resourceTree),
    [resourceTree, entities]
  );

  const highlightedItemIds = useHighlightedItemIds(rootItem);

  const resourceTypeOptions = useMemo(() => collectResourceTypesFromTree([rootItem]), [rootItem]);

  const [rootResourceType, setRootResourceType] = useState<string>(resourceTypeOptions[0] || '');

  const rootResourceGroupId = useMemo(() => getRootResourceGroupId(resourceTree), [resourceTree]);

  const { expandedIds, handleExpandChange } = useExpandedIds(rootItem.id);

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
      const request: SingleTimelineRequest<QueryFilter, TaskFilter> = {
        entry: {
          ResourceGroup: {
            resource_group_id: rootResourceGroupId!,
            resource_type_name: rootResourceType,
            long_entities_threshold_s: null,
            entity_filter: { entity_type_name: null },
            app_params: { operator_id: null },
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

  const treeData = useMemo(() => [rootItem], [rootItem]);

  const columns = useMemo(() => {
    return [
      {
        key: 'resource',
        label: 'Resource',
        widthIndex: 0,
        isFirst: true,
        render: ({ item }: { item: TreeTableItem; level: number }) => {
          const selectedType = selectedTypes.get(item.id) || item.availableResourceTypes?.[0] || '';
          const availableFsmTypes = selectedType
            ? entities.resource_types[selectedType]?.used_by
            : undefined;
          return (
            <ResourceColumn
              item={item}
              selectedType={selectedType}
              onTypeChange={(itemId, newType) => {
                setSelectedTypes(prev => new Map(prev).set(itemId, newType));
                if (itemId === rootItem.id) {
                  setRootResourceType(newType);
                }
              }}
              availableFsmTypes={availableFsmTypes}
              selectedFsmType={selectedFsmTypes.get(item.id) ?? null}
              onFsmChange={(itemId, fsmType) => {
                setSelectedFsmTypes(prev => new Map(prev).set(itemId, fsmType));
              }}
            />
          );
        },
      },
      {
        key: 'usage',
        label: 'Usage',
        widthIndex: 1,
        subHeaderContent: (
          <div className="h-full overflow-hidden flex items-center py-2">
            <TimelineController
              startTime={startTime}
              durationSeconds={durationSeconds}
              timelineData={fetchedRootTimeline}
              onZoomChange={handleZoomChange}
            />
          </div>
        ),
        render: ({ item }: { item: TreeTableItem }) => (
          <UsageColumn
            item={item}
            engineId={engineId}
            queryBundle={queryBundle}
            selectedTypes={selectedTypes}
            selectedFsmTypes={selectedFsmTypes}
            startTime={startTime}
            durationSeconds={durationSeconds}
          />
        ),
      },
    ] satisfies Column<TreeTableItem>[];
  }, [
    startTime,
    durationSeconds,
    fetchedRootTimeline,
    selectedTypes,
    selectedFsmTypes,
    entities,
    rootItem,
    engineId,
    queryBundle,
    handleZoomChange,
  ]);

  return (
    <div className="flex flex-col h-full w-full">
      <TimelineToolbar durationSeconds={durationSeconds} />
      <div className="flex-1 min-h-0">
        <TreeTable<TreeTableItem>
          data={treeData}
          columns={columns}
          initialSelectedItemId={rootItem.id}
          columnWidths={[275, 'auto']}
          onExpandChange={onExpandChange}
          highlightedItemIds={highlightedItemIds}
        />
      </div>
    </div>
  );
}
