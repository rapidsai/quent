// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useMemo, useState } from 'react';
import { createFsmTypeColorFn } from '@quent/utils';
import type {
  EntityRef,
  FiniteStateMachine,
  NvtxRangeItem,
  QueryBundle,
  ZoomRange,
} from '@quent/utils';
import { EntityDetailDrawer } from '@/components/EntityDetailDrawer';
import { NvtxDetailDrawer } from '@/components/NvtxDetailDrawer';
import { THEME_DARK, THEME_LIGHT } from '@/contexts/ThemeContext';
import { useNvtxTreeModel } from './NvtxTree';
import { createLongEntitiesTimelineSubRow, createOperatorGanttTimelineSubRow } from './sub-rows';
import {
  useResourceTimelinesTreeModel,
  type ResourceTimelineSubRow,
} from './ResourceTimelinesTree';
import { TimelineTreeTable, useTimelineTreeSetup } from './TimelineTreeTable';
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
  const [drawerSpanId, setDrawerSpanId] = useState<number | null>(null);
  const toggleDrawerFsm = useCallback((fsm: FiniteStateMachine) => {
    setDrawerSpanId(null);
    setDrawerFsm(selectedFsm => (selectedFsm?.id === fsm.id ? null : fsm));
  }, []);
  const closeDrawer = useCallback(() => setDrawerFsm(null), []);
  const toggleDrawerSpan = useCallback((range: NvtxRangeItem) => {
    setDrawerFsm(null);
    setDrawerSpanId(selectedSpanId => (selectedSpanId === range.span_id ? null : range.span_id));
  }, []);
  const closeSpanDrawer = useCallback(() => setDrawerSpanId(null), []);

  const stateColorFn = useMemo(
    () => createFsmTypeColorFn(entities.fsm_types, isDark ? THEME_DARK : THEME_LIGHT),
    [entities.fsm_types, isDark]
  );
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
  const resourceTree = useResourceTimelinesTreeModel({
    engineId,
    queryBundle,
    isDark,
    subRows: resourceSubRows ?? defaultResourceSubRows,
    seedRootExpanded,
  });
  const nvtxTree = useNvtxTreeModel({
    engineId,
    queryBundle,
    isDark,
    selectedSpanId: drawerSpanId ?? undefined,
    onRangeSelect: toggleDrawerSpan,
    onBackgroundClick: closeSpanDrawer,
  });
  const highlightedItemIds = new Set([
    ...(resourceTree.highlightedItemIds ?? []),
    ...(nvtxTree.highlightedItemIds ?? []),
  ]);
  const hasFilterMatches = resourceTree.filterMatchCount + nvtxTree.filterMatchCount > 0;
  const trees =
    resourceTree.isFilterActive && resourceTree.showOthers && !hasFilterMatches
      ? []
      : [resourceTree, nvtxTree];

  return (
    <TimelineTreeTable
      durationSeconds={durationSeconds}
      isDark={isDark}
      trees={trees}
      controls={{ ...resourceTree, highlightedItemIds }}
    >
      <EntityDetailDrawer
        fsm={drawerFsm}
        resourceLabel={resourceLabel}
        operatorLabel={operatorLabel}
        onClose={closeDrawer}
        stateColorFn={stateColorFn}
        queryBundle={queryBundle}
      />
      <NvtxDetailDrawer
        contextId={nvtxTree.contextId ?? null}
        spanId={drawerSpanId}
        queryStartUnixNs={queryBundle.start_time_unix_ns}
        onClose={closeSpanDrawer}
        onSelectSpan={setDrawerSpanId}
      />
    </TimelineTreeTable>
  );
}
