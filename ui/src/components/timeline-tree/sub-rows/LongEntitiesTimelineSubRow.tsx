// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  LONG_ENTITIES_ROW_TYPE,
  longEntitiesRowId,
  resourceIdFromLongEntitiesRowId,
} from '@quent/components';
import {
  EntityTypeKey,
  type EntityRef,
  type FiniteStateMachine,
  type QueryBundle,
} from '@quent/utils';
import { LongEntitiesRow } from '@/components/LongEntitiesRow';
import {
  createSyntheticSubRow,
  createTimelineSubRow,
  createTimelineSubRowDisclosure,
  mapTreeItems,
} from './subRow';

interface LongEntitiesTimelineSubRowOptions {
  engineId: string;
  queryBundle: QueryBundle<EntityRef>;
  isDark: boolean;
  onEntitySelect?: (fsm: FiniteStateMachine) => void;
  selectedEntityId?: string;
  onBackgroundClick?: () => void;
  zeroUtilizationResourceIds?: ReadonlySet<string>;
}

export function createLongEntitiesTimelineSubRow({
  engineId,
  queryBundle,
  isDark,
  onEntitySelect,
  selectedEntityId,
  onBackgroundClick,
  zeroUtilizationResourceIds,
}: LongEntitiesTimelineSubRowOptions) {
  return createTimelineSubRow({
    id: 'long-entities',
    rowType: LONG_ENTITIES_ROW_TYPE,
    label: 'Entities',
    injectRows: rootItem =>
      mapTreeItems(rootItem, (item, children) => {
        if (item.type !== EntityTypeKey.Resource || zeroUtilizationResourceIds?.has(item.id)) {
          return children;
        }
        const chartRow = createSyntheticSubRow(longEntitiesRowId(item.id), LONG_ENTITIES_ROW_TYPE);
        return [
          ...children,
          createTimelineSubRowDisclosure(item.id, LONG_ENTITIES_ROW_TYPE, chartRow),
        ];
      }),
    renderTimeline: item => {
      const resourceId = resourceIdFromLongEntitiesRowId(item.id);
      if (resourceId == null) {
        return null;
      }
      return (
        <LongEntitiesRow
          engineId={engineId}
          queryId={queryBundle.query_id}
          resourceId={resourceId}
          durationSeconds={queryBundle.duration_s}
          isDark={isDark}
          onEntitySelect={onEntitySelect}
          selectedEntityId={selectedEntityId}
          onBackgroundClick={onBackgroundClick}
        />
      );
    },
  });
}
