// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Column, TreeTable } from '@/components/ui/tree-table';
import { useCallback, useMemo, useState } from 'react';
import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { useHydrateAtoms } from 'jotai/utils';
import { useHighlightedItemIds } from '@/hooks/useHighlightedItemIds';
import { ResourceTree } from '~quent/types/ResourceTree';
import { TimelineController } from './timeline/TimelineController';
import { collectResourceTypesFromTree } from '@/lib/resource.utils';
import { EntityRefKey } from '@/types';
import { TreeTableItem } from './resource-tree/types';
import { ResourceColumn } from './resource-tree/ResourceColumn';
import { UsageColumn } from './resource-tree/UsageColumn';
import { DEFAULT_TIMELINE_HEIGHT } from './timeline/types';
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
import { TimelineToolbar } from './timeline/TimelineToolbar';
import {
  OperatorGanttChart,
  OPERATOR_TIMELINE_ROW_TYPE,
  getWorkerIdsFromPlanTree,
  operatorTimelineRowId,
  operatorsWithActiveSpansForWorker,
  workerIdFromOperatorTimelineRowId,
} from './operator-timeline';

function getRootResourceGroupId(resourceTree: ResourceTree<EntityRef>): string | null {
  if (!('ResourceGroup' in resourceTree)) return null;
  const [, entityId] = Object.entries(resourceTree.ResourceGroup.id)[0] as [EntityRefKey, string];
  return entityId;
}

/** Create the synthetic operator-timeline row for a worker. Defaults to collapsed (no children). */
function createOperatorTimelineRow(workerId: string): TreeTableItem {
  return {
    id: operatorTimelineRowId(workerId),
    type: OPERATOR_TIMELINE_ROW_TYPE,
    entity: {} as TreeTableItem['entity'],
  };
}

/**
 * Inject an expandable "Operator timeline" row under each resource whose id matches a plan_tree worker.
 * Injected rows default to collapsed.
 *
 * If we have more than just operator timelines we should create a section for each of a certain type of
 * resource that can handle multiple tabbed sections, something like that.
 */
function injectOperatorTimelineRows(item: TreeTableItem, workerIds: Set<string>): TreeTableItem {
  const transformedChildren = item.children?.map(child =>
    injectOperatorTimelineRows(child, workerIds)
  );
  if (!workerIds.has(item.id)) {
    return transformedChildren?.length ? { ...item, children: transformedChildren } : { ...item };
  }
  const operatorTimelineRow = createOperatorTimelineRow(item.id);
  const children = [operatorTimelineRow, ...(transformedChildren ?? [])];
  return { ...item, children };
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

  const workerIdsFromPlanTree = useMemo(
    () => new Set(getWorkerIdsFromPlanTree(queryBundle.plan_tree)),
    [queryBundle.plan_tree]
  );

  const treeData = useMemo(
    () => [injectOperatorTimelineRows(rootItem, workerIdsFromPlanTree)],
    [rootItem, workerIdsFromPlanTree]
  );

  /** Operator entries per worker id (for expandable gantt under each worker resource). */
  const operatorEntriesByWorker = useMemo(() => {
    const map = new Map<string, ReturnType<typeof operatorsWithActiveSpansForWorker>>();
    for (const workerId of workerIdsFromPlanTree) {
      map.set(workerId, operatorsWithActiveSpansForWorker(queryBundle, startTime, workerId));
    }
    return map;
  }, [queryBundle, startTime, workerIdsFromPlanTree]);

  const columns = useMemo(() => {
    return [
      {
        key: 'resource',
        label: 'Resource',
        widthIndex: 0,
        isFirst: true,
        render: ({ item }: { item: TreeTableItem; level: number }) => {
          switch (item.type) {
            case OPERATOR_TIMELINE_ROW_TYPE: {
              return null;
            }
            default: {
              const selectedType =
                selectedTypes.get(item.id) || item.availableResourceTypes?.[0] || '';
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
            }
          }
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
        render: ({ item }: { item: TreeTableItem }) => {
          switch (item.type) {
            case OPERATOR_TIMELINE_ROW_TYPE: {
              const workerId = workerIdFromOperatorTimelineRowId(item.id);
              const operators =
                workerId != null ? (operatorEntriesByWorker.get(workerId) ?? []) : [];
              return (
                <OperatorGanttChart
                  operators={operators}
                  startTime={startTime}
                  durationSeconds={durationSeconds}
                  height={DEFAULT_TIMELINE_HEIGHT * 1.2}
                />
              );
            }
            default: {
              return (
                <UsageColumn
                  item={item}
                  engineId={engineId}
                  queryBundle={queryBundle}
                  selectedTypes={selectedTypes}
                  selectedFsmTypes={selectedFsmTypes}
                  startTime={startTime}
                  durationSeconds={durationSeconds}
                />
              );
            }
          }
        },
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
    operatorEntriesByWorker,
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
