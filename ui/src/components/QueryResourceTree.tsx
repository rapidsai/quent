// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Column, TreeTable } from '@quent/components';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { useAtom } from 'jotai';
import { useHighlightedItemIds, useBulkTimelines, useHydrateTimelineAtoms } from '@quent/hooks';
import { ResourceTree, QueryBundle, EntityTypeKey } from '@quent/utils';
import type { EntityRef, SingleTimelineRequest, QueryFilter, OperatorFilter } from '@quent/utils';
import { TimelineController, TimelineRuler } from '@quent/components';
import { collectResourceTypesFromTree } from '@quent/components';
import { EntityRefKey } from '@quent/utils';
import { TreeTableItem } from '@quent/components';
import { ResourceColumn } from '@quent/components';
import { UsageColumn } from '@quent/components';
import { DEFAULT_TIMELINE_HEIGHT } from '@quent/components';
import { fetchSingleTimeline, DEFAULT_STALE_TIME } from '@quent/client';
import {
  transformResourceTree,
  getAdaptiveNumBins,
  nanosToMs,
  collectVisibleEntries,
  buildBulkParamsForItem,
  findItemById,
} from '@quent/components';
import { useExpandedIds } from '@/hooks/useExpandedIds';
import {
  selectedTypesAtom,
  selectedFsmTypesAtom,
  rootResourceTypeAtom,
} from '@/atoms/resourceTree';
import { TimelineToolbar } from '@quent/components';
import { useTheme, THEME_DARK, THEME_LIGHT } from '@/contexts/ThemeContext';
import {
  OperatorGanttChart,
  OPERATOR_TIMELINE_ROW_TYPE,
  getWorkerIdsFromPlanTree,
  operatorTimelineRowId,
  operatorsWithActiveSpansForWorker,
  workerIdFromOperatorTimelineRowId,
} from '@quent/components';
import {
  LONG_ENTITIES_ROW_TYPE,
  longEntitiesRowId,
  resourceIdFromLongEntitiesRowId,
} from '@quent/components';
import { LongEntitiesRow } from '@/components/LongEntitiesRow';
import { EntityDetailDrawer } from '@/components/EntityDetailDrawer';
import type { FiniteStateMachine } from '@quent/utils';
import { createFsmTypeColorFn } from '@quent/utils';

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

/** Create the synthetic long-entities row for a leaf resource. */
function createLongEntitiesRow(resourceId: string): TreeTableItem {
  return {
    id: longEntitiesRowId(resourceId),
    type: LONG_ENTITIES_ROW_TYPE,
    entity: {} as TreeTableItem['entity'],
  };
}

function GanttRowLabel({ children }: { children: string }) {
  return (
    <span className="flex items-center">
      <span aria-hidden className="mr-4 h-4 w-4 shrink-0" />
      <span className="text-xs leading-none text-muted-foreground">{children}</span>
    </span>
  );
}

/**
 * Insert a long-entities row as a sibling immediately after each leaf resource,
 * so its compact Gantt is always shown below the resource (whenever in view)
 * rather than gated behind expansion. Leaf resources keep no synthetic children,
 * so they stay non-expandable. Groups (which aggregate resources) are untouched.
 */
function injectLongEntitiesRows(item: TreeTableItem): TreeTableItem {
  if (!item.children?.length) return { ...item };
  const children: TreeTableItem[] = [];
  for (const child of item.children) {
    children.push(injectLongEntitiesRows(child));
    if (child.type === EntityTypeKey.Resource) {
      children.push(createLongEntitiesRow(child.id));
    }
  }
  return { ...item, children };
}

interface QueryResourceTreeProps {
  engineId: string;
  queryBundle: QueryBundle<EntityRef>;
  initialZoomRange?: { start: number; end: number };
  seedRootExpanded?: boolean;
}

export function QueryResourceTree(props: QueryResourceTreeProps) {
  return <QueryResourceTreeContent {...props} />;
}

function QueryResourceTreeContent({
  queryBundle,
  engineId,
  initialZoomRange,
  seedRootExpanded = true,
}: QueryResourceTreeProps) {
  const { theme } = useTheme();
  const isDark = theme === THEME_DARK;
  const { entities, resource_tree: resourceTree } = queryBundle;
  const [selectedTypes, setSelectedTypes] = useAtom(selectedTypesAtom);
  const [selectedFsmTypes, setSelectedFsmTypes] = useAtom(selectedFsmTypesAtom);

  const [drawerFsm, setDrawerFsm] = useState<FiniteStateMachine | null>(null);
  const toggleDrawerFsm = useCallback(
    (fsm: FiniteStateMachine) =>
      setDrawerFsm(selectedFsm => (selectedFsm?.id === fsm.id ? null : fsm)),
    []
  );
  const closeDrawer = useCallback(() => setDrawerFsm(null), []);

  const stateColorFn = useMemo(
    () => createFsmTypeColorFn(entities.fsm_types, isDark ? THEME_DARK : THEME_LIGHT),
    [entities.fsm_types, isDark]
  );

  const resourceLabel = useCallback(
    (id: string) => {
      const r = entities.resources[id];
      return r ? `${r.instance_name} (${r.type_name})` : id;
    },
    [entities.resources]
  );
  const operatorLabel = useCallback(
    (id: string) => {
      const op = entities.operators[id];
      return op ? (op.instance_name ?? op.operator_type_name ?? id) : id;
    },
    [entities.operators]
  );

  const startTime = queryBundle.start_time_unix_ns;
  const durationSeconds = queryBundle.duration_s;
  const startTimeMs = useMemo(() => nanosToMs(startTime), [startTime]);
  const defaultZoomRange = { start: 0, end: durationSeconds };

  useHydrateTimelineAtoms({
    zoomRange: initialZoomRange ?? defaultZoomRange,
    debouncedZoomRange: initialZoomRange ?? defaultZoomRange,
    startTimeMs,
  });

  const rootItem = useMemo(
    () => transformResourceTree(entities, resourceTree),
    [resourceTree, entities]
  );

  const highlightedItemIds = useHighlightedItemIds(rootItem);

  const resourceTypeOptions = useMemo(() => collectResourceTypesFromTree([rootItem]), [rootItem]);

  const [rootResourceType, setRootResourceType] = useAtom(rootResourceTypeAtom);

  // Seed once per query when the atom is unset and options are available.
  useEffect(() => {
    if (rootResourceType != null) return;
    const initial = resourceTypeOptions[0];
    if (initial) setRootResourceType(initial);
  }, [rootResourceType, resourceTypeOptions, setRootResourceType]);

  const rootResourceGroupId = useMemo(() => getRootResourceGroupId(resourceTree), [resourceTree]);

  const { expandedIds, handleExpandChange } = useExpandedIds(
    seedRootExpanded ? rootItem.id : undefined
  );
  const controlledExpandedIds = expandedIds;

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

  const workerIdsFromPlanTree = useMemo(
    () => new Set(getWorkerIdsFromPlanTree(queryBundle.plan_tree)),
    [queryBundle.plan_tree]
  );

  const treeData = useMemo(
    () => [injectLongEntitiesRows(injectOperatorTimelineRows(rootItem, workerIdsFromPlanTree))],
    [rootItem, workerIdsFromPlanTree]
  );

  /** Operator entries per worker id (for expandable gantt under each worker resource). */
  const operatorEntriesByWorker = useMemo(() => {
    const map = new Map<string, ReturnType<typeof operatorsWithActiveSpansForWorker>>();
    for (const workerId of workerIdsFromPlanTree) {
      map.set(workerId, operatorsWithActiveSpansForWorker(queryBundle, workerId));
    }
    return map;
  }, [queryBundle, workerIdsFromPlanTree]);

  const columns = useMemo(() => {
    return [
      {
        key: 'resource',
        label: 'Resource',
        widthIndex: 0,
        isFirst: true,
        headerContent: (
          <div className="flex items-center h-full px-3 text-xs font-semibold text-muted-foreground select-none">
            Resource
          </div>
        ),
        render: ({ item }: { item: TreeTableItem; level: number }) => {
          switch (item.type) {
            case OPERATOR_TIMELINE_ROW_TYPE: {
              return <GanttRowLabel>Operators</GanttRowLabel>;
            }
            case LONG_ENTITIES_ROW_TYPE: {
              return <GanttRowLabel>Entities</GanttRowLabel>;
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
        headerContent: (
          <div className="h-full overflow-hidden flex items-center py-1">
            <TimelineController
              durationSeconds={durationSeconds}
              timelineData={fetchedRootTimeline}
              onZoomChange={handleZoomChange}
              isDark={isDark}
            />
          </div>
        ),
        subHeaderContent: <TimelineRuler isDark={isDark} />,
        render: ({ item }: { item: TreeTableItem }) => {
          switch (item.type) {
            case OPERATOR_TIMELINE_ROW_TYPE: {
              const workerId = workerIdFromOperatorTimelineRowId(item.id);
              const operators =
                workerId != null ? (operatorEntriesByWorker.get(workerId) ?? []) : [];
              return (
                <OperatorGanttChart
                  operators={operators}
                  durationSeconds={durationSeconds}
                  height={DEFAULT_TIMELINE_HEIGHT}
                  isDark={isDark}
                />
              );
            }
            case LONG_ENTITIES_ROW_TYPE: {
              const resourceId = resourceIdFromLongEntitiesRowId(item.id);
              if (resourceId == null) return null;
              return (
                <LongEntitiesRow
                  engineId={engineId}
                  queryId={queryBundle.query_id}
                  resourceId={resourceId}
                  durationSeconds={durationSeconds}
                  fsmTypes={entities.fsm_types}
                  isDark={isDark}
                  onEntitySelect={toggleDrawerFsm}
                  selectedEntityId={drawerFsm?.id}
                  onBackgroundClick={closeDrawer}
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
                  durationSeconds={durationSeconds}
                  isDark={isDark}
                />
              );
            }
          }
        },
      },
    ] satisfies Column<TreeTableItem>[];
  }, [
    durationSeconds,
    fetchedRootTimeline,
    isDark,
    selectedTypes,
    setSelectedTypes,
    selectedFsmTypes,
    setSelectedFsmTypes,
    setRootResourceType,
    entities,
    rootItem,
    engineId,
    queryBundle,
    handleZoomChange,
    operatorEntriesByWorker,
    toggleDrawerFsm,
    drawerFsm?.id,
    closeDrawer,
  ]);

  return (
    <div className="flex min-w-0 flex-col h-full w-full">
      <TimelineToolbar durationSeconds={durationSeconds} />
      <div className="min-w-0 flex-1 min-h-0">
        <TreeTable<TreeTableItem>
          data={treeData}
          columns={columns}
          initialSelectedItemId={rootItem.id}
          columnWidths={[275, 'auto']}
          onExpandChange={onExpandChange}
          highlightedItemIds={highlightedItemIds}
          controlledExpandedIds={controlledExpandedIds}
          virtualized
          // Estimate for virtualization
          rowHeight={DEFAULT_TIMELINE_HEIGHT}
        />
      </div>
      <EntityDetailDrawer
        fsm={drawerFsm}
        resourceLabel={resourceLabel}
        operatorLabel={operatorLabel}
        onClose={closeDrawer}
        stateColorFn={stateColorFn}
        queryBundle={queryBundle}
      />
    </div>
  );
}
