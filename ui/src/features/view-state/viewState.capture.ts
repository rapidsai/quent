// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { LONG_ENTITY_DENSITY_DEFAULT, type SerializableViewState } from '@quent/hooks';
import { DAG_LAYOUT_DIRECTION, SCALE_TYPE, NODE_LABEL_FIELD, type ZoomRange } from '@quent/utils';
import type { EntitiesTableState } from '@/atoms/entitiesTable';
import {
  DEFAULT_OPERATOR_TABLE_ENABLED,
  OPERATOR_TABLE_INDEX_ORDER,
} from '@/components/operator-table/types';
import {
  DEFAULT_PAGE_SIZE,
  SORT_DESC,
  defaultEntityFilters,
  normalizePageSize,
  parseOptionalNumber,
  validateEntityFilters,
} from '@/components/entities-table/utils';
import {
  OperatorGroupSchema,
  type ViewStateEntitiesState,
  type ViewStateV3,
} from './viewState.schema';

export function nonEmpty<T extends object>(value: T): T | undefined {
  return Object.keys(value).length > 0 ? value : undefined;
}

function isOperatorGroup(value: string): value is (typeof OperatorGroupSchema.options)[number] {
  return OperatorGroupSchema.validate(value);
}

export function captureTimeline(
  range: ZoomRange | null,
  shared: SerializableViewState['timeline']
): ViewStateV3['timeline'] {
  return nonEmpty({
    ...(range ? { zoomRange: range } : {}),
    ...(shared.longEntityDensity !== LONG_ENTITY_DENSITY_DEFAULT
      ? { longEntityDensity: shared.longEntityDensity }
      : {}),
  });
}

export function captureSelection(
  shared: SerializableViewState['selection']
): ViewStateV3['selection'] {
  return nonEmpty({
    ...(shared.planId ? { planId: shared.planId } : {}),
    ...(shared.pipes?.length ? { pipes: [...shared.pipes] } : {}),
    ...(shared.operatorNodeIds.length > 0 ? { operatorNodeIds: shared.operatorNodeIds } : {}),
  });
}

export function captureDag(shared: SerializableViewState['dag']): ViewStateV3['dag'] {
  return nonEmpty({
    ...(shared.nodeColorField ? { nodeColorField: shared.nodeColorField } : {}),
    ...(shared.nodeColorPalette !== 'blue' ? { nodeColorPalette: shared.nodeColorPalette } : {}),
    ...(shared.edgeWidthField ? { edgeWidthField: shared.edgeWidthField } : {}),
    ...(shared.edgeColorField ? { edgeColorField: shared.edgeColorField } : {}),
    ...(shared.edgeColorPalette !== 'teal' ? { edgeColorPalette: shared.edgeColorPalette } : {}),
    ...(shared.nodeLabelField !== NODE_LABEL_FIELD.NAME
      ? { nodeLabelField: shared.nodeLabelField }
      : {}),
    ...(shared.layoutDirection !== DAG_LAYOUT_DIRECTION.BOTTOM_TO_TOP
      ? { layoutDirection: shared.layoutDirection }
      : {}),
    ...(shared.scaleType !== SCALE_TYPE.LOG ? { scaleType: shared.scaleType } : {}),
  });
}

export function captureDataFlow(
  shared: SerializableViewState['dataFlow'],
  range: ZoomRange | null
): ViewStateV3['dataFlow'] {
  // A playhead sitting at the viewport start is the default, so it is not shared.
  const hasCustomPlayhead =
    shared.playheadS !== null &&
    (!range || Math.abs(shared.playheadS - range.start) > Number.EPSILON);
  return nonEmpty({
    ...(!shared.enabled ? { enabled: false } : {}),
    ...(shared.measure ? { measure: shared.measure } : {}),
    ...(shared.labelMeasure ? { labelMeasure: shared.labelMeasure } : {}),
    ...(shared.dimensions && shared.dimensions.length > 0 ? { dimensions: shared.dimensions } : {}),
    ...(hasCustomPlayhead && shared.playheadS !== null ? { playheadS: shared.playheadS } : {}),
  });
}

export function captureOperatorTable(
  shared: SerializableViewState['operatorTable']
): ViewStateV3['operatorTable'] {
  const groupingOrder = shared.groupingOrder?.filter(isOperatorGroup);
  const enabledGroups = shared.enabledGroups?.filter(isOperatorGroup);
  const hasCustomGroups =
    enabledGroups &&
    !OPERATOR_TABLE_INDEX_ORDER.every(
      key => enabledGroups.includes(key) === DEFAULT_OPERATOR_TABLE_ENABLED[key]
    );
  return nonEmpty({
    ...(groupingOrder ? { groupingOrder } : {}),
    ...(hasCustomGroups ? { enabledGroups } : {}),
    ...(shared.visibleStats !== undefined ? { visibleStats: shared.visibleStats } : {}),
    ...(shared.aggregation !== undefined ? { aggregation: shared.aggregation } : {}),
    ...(shared.sort !== undefined ? { sort: shared.sort } : {}),
  });
}

/** Returns `null` when the entity filters are invalid and cannot be shared. */
export function captureEntitiesState(
  tableState: EntitiesTableState,
  durationSeconds: number
): ViewStateEntitiesState | null {
  const filters = tableState.filters ?? defaultEntityFilters(durationSeconds);
  if (validateEntityFilters(filters).errors.length > 0) {
    return null;
  }
  const minUsageS = parseOptionalNumber(filters.minUsageS);
  const windowStart = parseOptionalNumber(filters.windowStart) ?? 0;
  const windowEnd = parseOptionalNumber(filters.windowEnd) ?? durationSeconds;
  const pageSize = normalizePageSize(filters.pageSize);
  const selectedEntityId = tableState.selectedEntityId ?? tableState.selected?.id;
  return {
    ...(filters.entityType ? { entityType: filters.entityType } : {}),
    ...(filters.resourceId ? { resourceId: filters.resourceId } : {}),
    ...(minUsageS !== null ? { minUsageS } : {}),
    ...(windowStart !== 0 || windowEnd !== durationSeconds
      ? { window: { start: windowStart, end: windowEnd } }
      : {}),
    ...(filters.sortDir !== SORT_DESC ? { sortDir: filters.sortDir } : {}),
    ...(pageSize !== DEFAULT_PAGE_SIZE ? { pageSize } : {}),
    ...(tableState.page > 0 ? { page: tableState.page } : {}),
    ...(selectedEntityId ? { selectedEntityId } : {}),
  };
}
