// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Column, TreeTable } from '@/components/ui/tree-table';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { useAtomValue, useStore } from 'jotai';
import { useHydrateAtoms } from 'jotai/utils';
import { useHighlightedItemIds } from '@/hooks/useHighlightedItemIds';
import { ResourceTree } from '~quent/types/ResourceTree';
import { TimelineController } from './timeline/TimelineController';
import { collectResourceTypesFromTree } from '@/lib/resource.utils';
import { EntityRefKey } from '@/types';
import { TreeTableItem } from './resource-tree/types';
import { ResourceColumn } from './resource-tree/ResourceColumn';
import { UsageColumn } from './resource-tree/UsageColumn';
import { QueryBundle } from '~quent/types/QueryBundle';
import type { EntityRef } from '~quent/types/EntityRef';
import { fetchSingleTimeline, DEFAULT_STALE_TIME } from '@/services/api';
import type { SingleTimelineRequest } from '~quent/types/SingleTimelineRequest';
import type { QueryFilter } from '~quent/types/QueryFilter';
import type { TaskFilter } from '~quent/types/TaskFilter';
import { transformResourceTree, getAdaptiveNumBins, nanosToMs } from '@/lib/timeline.utils';
import { useExpandedIds } from '@/hooks/useExpandedIds';
import { useBulkTimelines } from '@/hooks/useBulkTimelines';
import {
  zoomRangeAtom,
  debouncedZoomRangeAtom,
  startTimeMsAtom,
  timelineCacheKey,
  timelineDataAtom,
} from '@/atoms/timeline';
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

  useHydrateAtoms([
    [zoomRangeAtom, { start: 0, end: durationSeconds }],
    [debouncedZoomRangeAtom, { start: 0, end: durationSeconds }],
    [startTimeMsAtom, startTimeMs],
  ]);

  const rootItem = useMemo(
    () => transformResourceTree(entities, resourceTree),
    [resourceTree, entities]
  );

  const highlightedItemIds = useHighlightedItemIds(rootItem);

  const resourceTypeOptions = useMemo(() => collectResourceTypesFromTree([rootItem]), [rootItem]);

  const [rootResourceType, setRootResourceType] = useState<string>(resourceTypeOptions[0] || '');

  const rootResourceGroupId = useMemo(() => getRootResourceGroupId(resourceTree), [resourceTree]);

  // Atom cache key with fsmTypeName: null — TimelineController always shows the all-FSM aggregate
  const rootTimelineCacheKey = useMemo(
    () =>
      timelineCacheKey({
        resourceId: rootResourceGroupId ?? '',
        resourceTypeName: rootResourceType,
        fsmTypeName: null,
      }),
    [rootResourceGroupId, rootResourceType]
  );

  const store = useStore();

  const { expandedIds, handleExpandChange } = useExpandedIds(rootItem.id);

  const { handleZoomChange, handleExpand } = useBulkTimelines({
    engineId,
    queryId: queryBundle.query_id,
    rootItem,
    expandedIds,
    selectedTypes,
    groupFsmFilters: selectedFsmTypes,
    entities,
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

  // Store fetched full-range data into the atom cache under the FSM_ALL key.
  // Atom holds the previous value until overwritten, so keepPreviousData is unnecessary.
  useEffect(() => {
    if (fetchedRootTimeline) {
      store.set(timelineDataAtom(rootTimelineCacheKey), fetchedRootTimeline);
    }
  }, [fetchedRootTimeline, rootTimelineCacheKey, store]);

  const rootTimelineData = useAtomValue(timelineDataAtom(rootTimelineCacheKey));

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
              timelineData={rootTimelineData}
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
    rootTimelineData,
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
