// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Column, TreeTable } from '@/components/ui/tree-table';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { useAtom, useAtomValue } from 'jotai';
import { useHydrateAtoms } from 'jotai/utils';
import { useNavigate } from '@tanstack/react-router';
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
import { zoomRangeAtom, debouncedZoomRangeAtom, startTimeMsAtom } from '@/atoms/timeline';
import { expandedIdsAtom, selectedTypesAtom, selectedFsmTypesAtom } from '@/atoms/resourceTree';
import { TimelineToolbar } from './timeline/TimelineToolbar';
import { encodeTreeState } from '@/lib/treeStateParam';
import type { TreeState } from '@/lib/treeStateParam';

function getRootResourceGroupId(resourceTree: ResourceTree<EntityRef>): string | null {
  if (!('ResourceGroup' in resourceTree)) return null;
  const [, entityId] = Object.entries(resourceTree.ResourceGroup.id)[0] as [EntityRefKey, string];
  return entityId;
}

interface QueryResourceTreeProps {
  engineId: string;
  queryBundle: QueryBundle<EntityRef>;
  initialZoom?: { start: number; end: number };
  initialTreeState?: TreeState | null;
}

export function QueryResourceTree(props: QueryResourceTreeProps) {
  return <QueryResourceTreeContent {...props} />;
}

function QueryResourceTreeContent({
  queryBundle,
  engineId,
  initialZoom,
  initialTreeState,
}: QueryResourceTreeProps) {
  const { entities, resource_tree: resourceTree } = queryBundle;

  const startTime = queryBundle.start_time_unix_ns;
  const durationSeconds = queryBundle.duration_s;
  const startTimeMs = useMemo(() => nanosToMs(startTime), [startTime]);

  const rootItem = useMemo(
    () => transformResourceTree(entities, resourceTree),
    [resourceTree, entities]
  );

  const zoomInit = initialZoom ?? { start: 0, end: durationSeconds };

  const initialExpandedIds = useMemo(
    () =>
      initialTreeState ? new Set(initialTreeState.expandedIds) : new Set<string>([rootItem.id]),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    []
  );
  const initialSelectedTypes = useMemo(
    () =>
      initialTreeState
        ? new Map(Object.entries(initialTreeState.selectedTypes))
        : new Map<string, string>(),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    []
  );
  const initialSelectedFsmTypes = useMemo(
    () =>
      initialTreeState
        ? new Map(Object.entries(initialTreeState.selectedFsmTypes))
        : new Map<string, string | null>(),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    []
  );

  useHydrateAtoms([
    [zoomRangeAtom, zoomInit],
    [debouncedZoomRangeAtom, zoomInit],
    [startTimeMsAtom, startTimeMs],
    [expandedIdsAtom, initialExpandedIds],
    [selectedTypesAtom, initialSelectedTypes],
    [selectedFsmTypesAtom, initialSelectedFsmTypes],
  ]);

  const [selectedTypes, setSelectedTypes] = useAtom(selectedTypesAtom);
  const [selectedFsmTypes, setSelectedFsmTypes] = useAtom(selectedFsmTypesAtom);
  const expandedIds = useAtomValue(expandedIdsAtom);

  const highlightedItemIds = useHighlightedItemIds(rootItem);

  const resourceTypeOptions = useMemo(() => collectResourceTypesFromTree([rootItem]), [rootItem]);

  const [rootResourceType, setRootResourceType] = useState<string>(resourceTypeOptions[0] || '');

  const rootResourceGroupId = useMemo(() => getRootResourceGroupId(resourceTree), [resourceTree]);

  const { handleExpandChange } = useExpandedIds();
  const expandedIdsArray = useMemo(() => [...expandedIds], [expandedIds]);

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

  const navigate = useNavigate({ from: '/profile/engine/$engineId/query/$queryId/' });

  useEffect(() => {
    const timer = setTimeout(() => {
      const encoded = encodeTreeState({ expandedIds, selectedTypes, selectedFsmTypes });
      void navigate({
        search: prev => ({ ...prev, treeState: encoded }),
        replace: true,
      });
    }, 400);
    return () => clearTimeout(timer);
  }, [expandedIds, selectedTypes, selectedFsmTypes, navigate]);

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
    setSelectedTypes,
    setSelectedFsmTypes,
  ]);

  return (
    <div className="flex flex-col h-full w-full">
      <TimelineToolbar durationSeconds={durationSeconds} />
      <div className="flex-1 min-h-0">
        <TreeTable<TreeTableItem>
          data={treeData}
          columns={columns}
          initialSelectedItemId={rootItem.id}
          expandedItemIds={expandedIdsArray}
          columnWidths={[275, 'auto']}
          onExpandChange={onExpandChange}
          highlightedItemIds={highlightedItemIds}
        />
      </div>
    </div>
  );
}
