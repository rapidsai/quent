// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useMemo, useState } from 'react';
import { createFsmTypeColorFn } from '@quent/utils';
import type { EntityRef, FiniteStateMachine, QueryBundle, ZoomRange } from '@quent/utils';
import { EntityDetailDrawer } from '@/components/EntityDetailDrawer';
import { useNvtxTreeModel } from './NvtxTree';
import { createLongEntitiesTimelineSubRow, createOperatorGanttTimelineSubRow } from './sub-rows';
import {
  useResourceTimelinesTreeModel,
  type ResourceTimelineSubRow,
} from './ResourceTimelinesTree';
import { TimelineTreeTable, useTimelineTreeSetup } from './TimelineTreeTable';

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

  const stateColorFn = useMemo(
    () => createFsmTypeColorFn(entities.fsm_types, isDark ? 'dark' : 'light'),
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
  const longEntitiesSubRow = useMemo(
    () =>
      createLongEntitiesTimelineSubRow({
        engineId,
        queryBundle,
        isDark,
        onEntitySelect: toggleDrawerFsm,
        selectedEntityId: drawerFsm?.id,
        onBackgroundClick: closeDrawer,
      }),
    [closeDrawer, drawerFsm?.id, engineId, isDark, queryBundle, toggleDrawerFsm]
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
  const nvtxTree = useNvtxTreeModel({ engineId, queryBundle, isDark });

  return (
    <TimelineTreeTable
      durationSeconds={durationSeconds}
      isDark={isDark}
      trees={[resourceTree, nvtxTree]}
      controls={resourceTree}
    >
      <EntityDetailDrawer
        fsm={drawerFsm}
        resourceLabel={resourceLabel}
        operatorLabel={operatorLabel}
        onClose={closeDrawer}
        stateColorFn={stateColorFn}
        queryBundle={queryBundle}
      />
    </TimelineTreeTable>
  );
}
