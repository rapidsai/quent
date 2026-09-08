// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  LONG_ENTITIES_ROW_TYPE,
  longEntitiesRowId,
  resourceIdFromLongEntitiesRowId,
  type TreeTableItem,
} from '@quent/components';
import {
  EntityTypeKey,
  type EntityRef,
  type FiniteStateMachine,
  type QueryBundle,
} from '@quent/utils';
import { LongEntitiesRow } from '@/components/LongEntitiesRow';
import { createSyntheticSubRow, createTimelineSubRow, mapTreeItems } from './subRow';

interface LongEntitiesTimelineSubRowOptions {
  engineId: string;
  queryBundle: QueryBundle<EntityRef>;
  isDark: boolean;
  onEntitySelect?: (fsm: FiniteStateMachine) => void;
  selectedEntityId?: string;
  onBackgroundClick?: () => void;
}

export function createLongEntitiesTimelineSubRow({
  engineId,
  queryBundle,
  isDark,
  onEntitySelect,
  selectedEntityId,
  onBackgroundClick,
}: LongEntitiesTimelineSubRowOptions) {
  return createTimelineSubRow({
    id: 'long-entities',
    rowType: LONG_ENTITIES_ROW_TYPE,
    label: 'Entities',
    injectRows: rootItem =>
      mapTreeItems(rootItem, (_item, children) => {
        const next: TreeTableItem[] = [];
        for (const child of children) {
          next.push(child);
          if (child.type === EntityTypeKey.Resource) {
            next.push(createSyntheticSubRow(longEntitiesRowId(child.id), LONG_ENTITIES_ROW_TYPE));
          }
        }
        return next;
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
          fsmTypes={queryBundle.entities.fsm_types}
          isDark={isDark}
          onEntitySelect={onEntitySelect}
          selectedEntityId={selectedEntityId}
          onBackgroundClick={onBackgroundClick}
        />
      );
    },
  });
}
