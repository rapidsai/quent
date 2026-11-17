// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useMemo, useState } from 'react';
import type { EntityRef, FiniteStateMachine, QueryBundle, ZoomRange } from '@quent/utils';
import { EntityDetailDrawer } from '@/components/EntityDetailDrawer';
import { useNvtxTreeModel } from './NvtxTree';
import { createLongEntitiesTimelineSubRow, createOperatorGanttTimelineSubRow } from './sub-rows';
import {
  useResourceTimelinesTreeModel,
  type ResourceTimelineSubRow,
} from './ResourceTimelinesTree';
import { TimelineTreeFilterControls } from './TimelineTreeFilterControls';
import { TimelineTreeTable, useTimelineTreeSetup } from './TimelineTreeTable';
import {
  combineTimelineTreeFilters,
  useCombinedTimelineTreePinning,
} from './timelineTreeComposition';
import { useFullDurationZeroUtilizationResourceIds } from './useFullDurationZeroUtilizationResourceIds';

export interface QueryResourceTreeProps {
  engineId: string;
  queryBundle: QueryBundle<EntityRef>;
  resourceSubRows?: readonly ResourceTimelineSubRow[];
  initialZoomRange?: ZoomRange;
  seedRootExpanded?: boolean;
}

export function QueryResourceTree({
  queryBundle,
  engineId,
  resourceSubRows,
  initialZoomRange,
  seedRootExpanded = true,
}: QueryResourceTreeProps) {
  const { durationSeconds, isDark } = useTimelineTreeSetup(queryBundle, initialZoomRange);
  const { entities } = queryBundle;

  const [drawerFsm, setDrawerFsm] = useState<FiniteStateMachine | null>(null);
  const toggleDrawerFsm = useCallback(
    (fsm: FiniteStateMachine) =>
      setDrawerFsm(selectedFsm => (selectedFsm?.id === fsm.id ? null : fsm)),
    []
  );
  const closeDrawer = useCallback(() => setDrawerFsm(null), []);

  const resourceLabel = useCallback(
    (id: string) => {
      const resource = entities.resources[id];
      return resource ? `${resource.instance_name} (${resource.type_name})` : id;
    },
    [entities.resources]
  );
  const operatorLabel = useCallback(
    (id: string) => {
      const operator = entities.operators[id];
      return operator ? (operator.instance_name ?? operator.operator_type_name ?? id) : id;
    },
    [entities.operators]
  );

  const operatorGanttSubRow = useMemo(
    () => createOperatorGanttTimelineSubRow({ queryBundle, isDark }),
    [isDark, queryBundle]
  );
  const zeroUtilizationResourceIds = useFullDurationZeroUtilizationResourceIds(
    engineId,
    queryBundle.query_id,
    queryBundle.duration_s,
    entities,
    resourceSubRows === undefined
  );
  const longEntitiesSubRow = useMemo(
    () =>
      createLongEntitiesTimelineSubRow({
        engineId,
        queryBundle,
        isDark,
        onEntitySelect: toggleDrawerFsm,
        selectedEntityId: drawerFsm?.id,
        onBackgroundClick: closeDrawer,
        zeroUtilizationResourceIds,
      }),
    [
      closeDrawer,
      drawerFsm?.id,
      engineId,
      isDark,
      queryBundle,
      toggleDrawerFsm,
      zeroUtilizationResourceIds,
    ]
  );
  const defaultResourceSubRows = useMemo(
    () => [operatorGanttSubRow, longEntitiesSubRow],
    [longEntitiesSubRow, operatorGanttSubRow]
  );
  const nvtxTree = useNvtxTreeModel({ engineId, queryBundle, isDark });
  const resourceTree = useResourceTimelinesTreeModel({
    engineId,
    queryBundle,
    isDark,
    subRows: resourceSubRows ?? defaultResourceSubRows,
    seedRootExpanded,
  });
  const combinedFilter = combineTimelineTreeFilters([resourceTree.filter, nvtxTree.filter]);
  const pinningSources = useMemo(
    () => [resourceTree.pinningSource, nvtxTree.pinningSource],
    [nvtxTree.pinningSource, resourceTree.pinningSource]
  );
  const { pinning, pinMany } = useCombinedTimelineTreePinning(pinningSources);
  const filters = (
    <TimelineTreeFilterControls
      {...resourceTree.filterOptions}
      matchCount={combinedFilter.matchCount}
      matchIds={combinedFilter.pinnableMatchIds}
      onPinMany={pinMany}
      pinnedIds={pinning.ids}
    />
  );
  const highlightedItemIds = new Set([
    ...(resourceTree.highlightedItemIds ?? []),
    ...(nvtxTree.highlightedItemIds ?? []),
  ]);
  const indicatorItemIds = new Set([
    ...(resourceTree.indicatorItemIds ?? []),
    ...(nvtxTree.indicatorItemIds ?? []),
  ]);
  const trees =
    combinedFilter.isActive && resourceTree.showOthers && combinedFilter.matchCount === 0
      ? []
      : [resourceTree, nvtxTree];

  return (
    <TimelineTreeTable
      durationSeconds={durationSeconds}
      isDark={isDark}
      trees={trees}
      controls={{ ...resourceTree, filters, pinning, highlightedItemIds, indicatorItemIds }}
    >
      <EntityDetailDrawer
        fsm={drawerFsm}
        resourceLabel={resourceLabel}
        operatorLabel={operatorLabel}
        onClose={closeDrawer}
        queryBundle={queryBundle}
      />
    </TimelineTreeTable>
  );
}
