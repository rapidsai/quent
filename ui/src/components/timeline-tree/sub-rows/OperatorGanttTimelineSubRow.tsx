// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  DEFAULT_TIMELINE_HEIGHT,
  OPERATOR_TIMELINE_ROW_TYPE,
  OperatorGanttChart,
  getWorkerIdsFromPlanTree,
  operatorTimelineRowId,
  operatorsWithActiveSpansForWorker,
  workerIdFromOperatorTimelineRowId,
} from '@quent/components';
import type { EntityRef, QueryBundle } from '@quent/utils';
import { createSyntheticSubRow, createTimelineSubRow, mapTreeItems } from './subRow';

interface OperatorGanttTimelineSubRowOptions {
  queryBundle: QueryBundle<EntityRef>;
  isDark: boolean;
}

export function createOperatorGanttTimelineSubRow({
  queryBundle,
  isDark,
}: OperatorGanttTimelineSubRowOptions) {
  const workerIds = new Set(getWorkerIdsFromPlanTree(queryBundle.plan_tree));
  const entriesByWorker = new Map<string, ReturnType<typeof operatorsWithActiveSpansForWorker>>();
  for (const workerId of workerIds) {
    entriesByWorker.set(workerId, operatorsWithActiveSpansForWorker(queryBundle, workerId));
  }

  return createTimelineSubRow({
    id: 'operator-gantt',
    rowType: OPERATOR_TIMELINE_ROW_TYPE,
    label: 'Operators',
    injectRows: rootItem =>
      mapTreeItems(rootItem, (item, children) =>
        workerIds.has(item.id)
          ? [
              createSyntheticSubRow(operatorTimelineRowId(item.id), OPERATOR_TIMELINE_ROW_TYPE),
              ...children,
            ]
          : children
      ),
    renderTimeline: item => {
      const workerId = workerIdFromOperatorTimelineRowId(item.id);
      const operators = workerId != null ? (entriesByWorker.get(workerId) ?? []) : [];
      return (
        <OperatorGanttChart
          operators={operators}
          durationSeconds={queryBundle.duration_s}
          height={DEFAULT_TIMELINE_HEIGHT}
          isDark={isDark}
        />
      );
    },
  });
}
