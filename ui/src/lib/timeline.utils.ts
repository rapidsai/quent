// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { TimelineSeries, TimelineMark } from '@/components/timeline/types';
import { TreeTableItem } from '@/components/resource-tree/types';
import { formatQuantity } from '@/services/formatters';
import { entityRefToEntitiesKey } from '@/lib/queryBundle.utils';
import { collectResourceTypesFromTree, getIconForType } from '@/lib/resource.utils';
import type { ResourceTimeline } from '~quent/types/ResourceTimeline';
import { QueryEntities } from '~quent/types/QueryEntities';
import { ResourceTree } from '~quent/types/ResourceTree';
import type { EntityRef } from '~quent/types/EntityRef';
import { EntityTypeValue, EntityRefKey, EntityTypeKey } from '@/types';
import type { QuantitySpec } from '~quent/types/QuantitySpec';
import type { CapacityDecl } from '~quent/types/CapacityDecl';
import type { EChartsInstance } from 'echarts-for-react';
import { connect, getInstanceByDom } from '@/lib/echarts';
import { CHART_GROUP } from '@/components/timeline/Timeline';
import { getColorForKey, lightenColor, WHITE, withOpacity } from '@/services/colors';
import type { BinnedSpanSec } from '~quent/types/BinnedSpanSec';
import type { SingleTimelineResponse } from '~quent/types/SingleTimelineResponse';
import type { FiniteStateMachine } from '~quent/types/FiniteStateMachine';
import type { TimelineRequest } from '~quent/types/TimelineRequest';
import type { TaskFilter } from '~quent/types/TaskFilter';
import type { TimelineConfig } from '~quent/types/TimelineConfig';

const MAX_TIMELINE_BINS = 400;
const LONG_ENTITIES_BIN_MULTIPLIER = 30;

/** Convert a nanosecond-precision bigint epoch to milliseconds, preserving sub-ms precision. */
export function nanosToMs(ns: bigint): number {
  return Number(ns / 1_000_000n) + Number(ns % 1_000_000n) / 1_000_000;
}

/**
 * Currently static but may be used in the future to prevent sub
 * nanosecond bin sizes
 */
export function getAdaptiveNumBins(): number {
  return MAX_TIMELINE_BINS;
}

/** Threshold for "long" entities: 10x the current bin duration in seconds. */
export function getLongEntitiesThreshold(windowSeconds: number): number {
  const numBins = getAdaptiveNumBins();
  return LONG_ENTITIES_BIN_MULTIPLIER * (windowSeconds / numBins);
}

export function buildBinnedTimelineSeries(
  data: ResourceTimeline,
  config: BinnedSpanSec,
  startTime: bigint,
  capacities?: CapacityDecl[],
  quantitySpecs?: { [key in string]?: QuantitySpec }
): {
  timestamps: number[];
  series: TimelineSeries;
} {
  const { bin_duration, num_bins, span } = config;

  const numBinsNumber = Number(num_bins);
  const firstBinMs = nanosToMs(startTime) + span.start * 1_000;
  const binDurationMs = bin_duration * 1_000;

  const timestamps = new Array<number>(numBinsNumber);
  for (let i = 0; i < numBinsNumber; i++) {
    timestamps[i] = firstBinMs + i * binDurationMs;
  }

  const getFormatter = (capacityName: string): ((value: number) => string) => {
    const capDecl = capacities?.find(c => c.name === capacityName);
    const spec = capDecl ? quantitySpecs?.[capDecl.quantity] : undefined;
    if (spec && capDecl) {
      return (value: number, decimals: number = 2) =>
        formatQuantity(value, spec, capDecl.kind, decimals);
    }
    return (value: number) => String(value);
  };

  // Build series based on data type
  const series: TimelineSeries = {};

  if ('Binned' in data) {
    // ResourceTimelineBinned: capacities_values (flat: capacity → values)
    const { capacities_values } = data.Binned;
    for (const [capacity, values] of Object.entries(capacities_values)) {
      const formatter = getFormatter(capacity);
      series[capacity] = {
        color: getColorForKey(capacity),
        formatter,
        values: values ?? [],
        binDuration: bin_duration,
      };
    }
  } else if ('BinnedByState' in data) {
    const { capacities_states_values } = data.BinnedByState;
    for (const capacityType of Object.keys(capacities_states_values)) {
      const capacityStateValues = capacities_states_values[capacityType] ?? {};
      for (const [state, values] of Object.entries(capacityStateValues)) {
        const formatter = getFormatter(capacityType);
        if (values) {
          series[state] = {
            color: getColorForKey(state),
            binDuration: bin_duration,
            formatter,
            values,
          };
        }
      }
    }
  }

  // Ensures the timeline is cleared when new "all 0" or "no series" data is received
  if (Object.keys(series).length === 0) {
    series['empty'] = {
      color: withOpacity(WHITE, 0),
      binDuration: bin_duration,
      formatter: (value: number) => String(value),
      values: [],
    };
  }
  return { timestamps, series };
}

/** Extract the config from a SingleTimelineResponse */
export function getTimelineConfig(response: SingleTimelineResponse): BinnedSpanSec {
  return response.config;
}

/** Extract long_fsms from a ResourceTimeline response. */
export function getLongFsms(data: ResourceTimeline): FiniteStateMachine[] {
  if ('Binned' in data) return data.Binned.long_fsms;
  if ('BinnedByState' in data) return data.BinnedByState.long_fsms;
  return [];
}

/**
 * Convert long_fsms into a flat array of timeline marks.
 * Each pair of consecutive transitions defines a time range for the state
 * entered by the first transition.
 * When resourceIdsForFilter is provided, only states that have at least one
 * usage on one of those resources are included (e.g. hide "queueing" on a resource lane).
 */
export function buildTimelineMarks(
  longFsms: FiniteStateMachine[],
  startTime: bigint,
  resourceIdsForFilter?: Set<string> | null
): TimelineMark[] | undefined {
  if (longFsms.length === 0) return undefined;

  const startTimeMs = nanosToMs(startTime);

  const marks = longFsms.flatMap(fsm => {
    const label = fsm.instance_name || fsm.id;
    return fsm.transitions
      .slice(0, -1)
      .map((transition, i) => {
        if (
          resourceIdsForFilter != null &&
          !transition.usages?.some(u => resourceIdsForFilter.has(u.resource))
        ) {
          return null;
        }
        const next = fsm.transitions[i + 1];
        const xStart = startTimeMs + transition.timestamp * 1000;
        const xEnd = startTimeMs + next.timestamp * 1000;
        return { label, stateName: transition.name, xStart, xEnd };
      })
      .filter((m): m is TimelineMark => m != null && m.xEnd > m.xStart);
  });

  return marks.length > 0 ? marks : undefined;
}

/**
 * Merge overlay series into base series for overlay rendering.
 * Each overlay series entry gets a lightened color, an `isOverlay` flag,
 * and a tooltip name of "{state} ({overlayLabel})".
 */
export function mergeOverlaySeries(
  baseSeries: TimelineSeries,
  overlaySeries: TimelineSeries,
  overlayLabel: string,
  lightenAmount: number
): TimelineSeries {
  const merged: TimelineSeries = { ...baseSeries };
  for (const [state, overlayEntry] of Object.entries(overlaySeries)) {
    const baseEntry = baseSeries[state];
    const baseColor = baseEntry?.color ?? overlayEntry.color;
    const overlayName = `${state} (${overlayLabel})`;
    merged[overlayName] = {
      ...overlayEntry,
      color: lightenColor(baseColor, lightenAmount),
      isOverlay: true,
    };
  }
  return merged;
}

/** Extract the resource_type_name from a TimelineRequest (empty string for Resource requests) */
export function getResourceTypeName(params: TimelineRequest<TaskFilter> | undefined): string {
  if (!params) return '';
  if ('ResourceGroup' in params) return params.ResourceGroup.resource_type_name;
  return '';
}

/** Extract the entity_type_name (FSM filter) from a TimelineRequest */
export function getFsmTypeName(params: TimelineRequest<TaskFilter>): string | null {
  if ('ResourceGroup' in params) return params.ResourceGroup.entity_filter.entity_type_name;
  return params.Resource.entity_filter.entity_type_name;
}

/** Clone entries and set operator_id on each TimelineRequest */
export function setOperatorOnEntry(
  entry: TimelineRequest<TaskFilter>,
  operatorId: string
): TimelineRequest<TaskFilter> {
  if ('ResourceGroup' in entry) {
    return {
      ResourceGroup: {
        ...entry.ResourceGroup,
        app_params: { ...entry.ResourceGroup.app_params, operator_id: operatorId },
      },
    };
  }
  return {
    Resource: {
      ...entry.Resource,
      application: { ...entry.Resource.application, operator_id: operatorId },
    },
  };
}

export function setOperatorOnEntries(
  baseEntries: Record<string, TimelineRequest<TaskFilter>>,
  operatorId: string
): Record<string, TimelineRequest<TaskFilter>> {
  return Object.fromEntries(
    Object.entries(baseEntries).map(([id, entry]) => [id, setOperatorOnEntry(entry, operatorId)])
  );
}

const SECOND_MS = 1_000;
const MINUTE_MS = 60 * SECOND_MS;
const HOUR_MS = 60 * MINUTE_MS;
const DAY_MS = 24 * HOUR_MS;

const NICE_TIMELINE_INTERVALS_MS = [
  100,
  200,
  500,
  1 * SECOND_MS,
  2 * SECOND_MS,
  5 * SECOND_MS,
  10 * SECOND_MS,
  15 * SECOND_MS,
  30 * SECOND_MS,
  1 * MINUTE_MS,
  2 * MINUTE_MS,
  5 * MINUTE_MS,
  10 * MINUTE_MS,
  15 * MINUTE_MS,
  30 * MINUTE_MS,
  1 * HOUR_MS,
  2 * HOUR_MS,
  3 * HOUR_MS,
  6 * HOUR_MS,
  12 * HOUR_MS,
  1 * DAY_MS,
  2 * DAY_MS,
  3 * DAY_MS,
  7 * DAY_MS,
  14 * DAY_MS,
  30 * DAY_MS,
] as const;

/**
 * Pick a nice x-axis interval for timeline charts based on visible span.
 * Supports short spans (seconds) through long spans (multi-day).
 * `targetSplits` is treated as the minimum number of displayed splits/labels.
 */
export function getTimelineXAxisIntervalMs(spanMs: number, targetSplits: number = 8): number {
  const safeSpanMs = Math.max(1, spanMs);
  const minSplits = Math.max(2, targetSplits);
  // To display at least `minSplits`, interval must be <= span/(minSplits-1).
  const maxAllowedStep = safeSpanMs / (minSplits - 1);

  // Choose the largest "nice" interval that still satisfies the minimum split count.
  for (let i = NICE_TIMELINE_INTERVALS_MS.length - 1; i >= 0; i--) {
    const intervalMs = NICE_TIMELINE_INTERVALS_MS[i]!;
    if (intervalMs <= maxAllowedStep) return intervalMs;
  }

  // Fallback for very small spans where even the smallest nice interval is too coarse.
  return maxAllowedStep;
}

function findExistingChartInGroup(chartGroup: string): EChartsInstance | null {
  const chartElements = document.querySelectorAll('[_echarts_instance_]');
  for (const el of chartElements) {
    const instance = getInstanceByDom(el as HTMLElement);
    if (instance && instance.group === chartGroup) {
      return instance as unknown as EChartsInstance;
    }
  }
  return null;
}

/**
 * Get the current zoom state from any existing chart in the group.
 * Returns null if no charts exist or no zoom state is set.
 */
export function getChartGroupZoomState(
  chartGroup: string = CHART_GROUP
): { start: number; end: number } | null {
  const existingInstance = findExistingChartInGroup(chartGroup);
  if (existingInstance) {
    const existingOption = existingInstance.getOption();
    const dataZoomOption = existingOption.dataZoom as Array<{ start?: number; end?: number }>;

    if (dataZoomOption?.[0]?.start !== undefined && dataZoomOption?.[0]?.end !== undefined) {
      return { start: dataZoomOption[0].start, end: dataZoomOption[0].end };
    }
  }
  return null;
}

export const connectChart = (
  instance: EChartsInstance,
  chartGroup: string = CHART_GROUP,
  activateBrushSelect = true
) => {
  // Apply current zoom to this chart without replacing its dataZoom components.
  // setOption({ dataZoom: [zoomState] }) would replace the array and break slider/inside config.
  const zoomState = getChartGroupZoomState(chartGroup);
  if (zoomState) {
    instance.dispatchAction({
      type: 'dataZoom',
      dataZoomIndex: 0,
      start: zoomState.start,
      end: zoomState.end,
    });
  }

  if (activateBrushSelect) {
    instance.dispatchAction({
      type: 'takeGlobalCursor',
      key: 'dataZoomSelect',
      dataZoomSelectActive: true,
    });
  }

  instance.group = chartGroup;
  connect(chartGroup);
};

/* Axis pointer sync — manual crosshair sync across charts
 *
 * We manually broadcast showTip/hideTip by converting a shared timestamp
 * to each chart's local pixel coordinate, since the controller uses a
 * different xAxis type (value) than the resource timelines (time).
 */

interface AxisPointerEntry {
  instance: EChartsInstance;
  xAxisIndex: number;
  onMouseMove: (e: { offsetX: number }) => void;
  onGlobalOut: () => void;
}

const axisPointerRegistry = new Set<AxisPointerEntry>();
let isBroadcasting = false;

function broadcastShowPointer(source: EChartsInstance, timestampMs: number) {
  if (isBroadcasting) return;
  isBroadcasting = true;
  try {
    axisPointerRegistry.forEach(({ instance, xAxisIndex }) => {
      if (instance === source) return;
      try {
        const pixel = instance.convertToPixel({ xAxisIndex }, timestampMs);
        if (pixel != null && isFinite(pixel)) {
          instance.dispatchAction({
            type: 'showTip',
            x: pixel,
            y: instance.getHeight() / 2,
          });
        }
      } catch {
        // Target chart may not be ready or value out of range
      }
    });
  } finally {
    isBroadcasting = false;
  }
}

function broadcastHidePointer(source: EChartsInstance) {
  if (isBroadcasting) return;
  isBroadcasting = true;
  try {
    axisPointerRegistry.forEach(({ instance }) => {
      if (instance === source) return;
      try {
        instance.dispatchAction({ type: 'hideTip' });
      } catch {
        // Ignore disposed instances
      }
    });
  } finally {
    isBroadcasting = false;
  }
}

/**
 * Register a chart instance for manual axis pointer sync.
 * Uses zr-level mouse events + convertFromPixel for reliable cross-chart sync
 * regardless of tooltip/axisPointer configuration differences.
 * @param xAxisIndex Which xAxis index carries the timestamp values (default 0).
 */
export function registerAxisPointerSync(instance: EChartsInstance, xAxisIndex = 0) {
  const onMouseMove = (e: { offsetX: number }) => {
    try {
      const value = instance.convertFromPixel({ xAxisIndex }, e.offsetX);
      if (value != null && isFinite(value as number)) {
        broadcastShowPointer(instance, value as number);
      }
    } catch {
      // Chart grid not ready
    }
  };

  const onGlobalOut = () => {
    broadcastHidePointer(instance);
  };

  const zr = instance.getZr();
  zr.on('mousemove', onMouseMove);
  zr.on('globalout', onGlobalOut);

  const entry = { instance, xAxisIndex, onMouseMove, onGlobalOut };
  axisPointerRegistry.add(entry);

  (instance as unknown as Record<string, unknown>).__axisPointerEntry = entry;
}

/** Unregister a chart instance from axis pointer sync. */
export function unregisterAxisPointerSync(instance: EChartsInstance) {
  const entry = (instance as unknown as Record<string, unknown>).__axisPointerEntry as
    | AxisPointerEntry
    | undefined;
  if (!entry) return;

  axisPointerRegistry.delete(entry);

  const zr = instance.getZr?.();
  if (zr) {
    zr.off('mousemove', entry.onMouseMove);
    zr.off('globalout', entry.onGlobalOut);
  }

  delete (instance as unknown as Record<string, unknown>).__axisPointerEntry;
}

// Helper function to lookup entity from QueryEntities
const lookupEntity = (
  entities: QueryEntities,
  entityType: EntityRefKey,
  entityId: string
): EntityTypeValue | undefined => {
  const entityKey = entityRefToEntitiesKey(entityType);
  if (!entityKey) return undefined; // handles Task and future unknown EntityRef variants

  const entityValue = entities[entityKey];

  // SingleEntity (Engine | Query | QueryGroup): single object with id
  if ('id' in entityValue && entityValue.id === entityId) {
    return entityValue as EntityTypeValue;
  }

  // Record<string, EntityTypeValue>: lookup by entityId key
  return (entityValue as Record<string, EntityTypeValue>)?.[entityId];
};

export const transformResourceTree = (
  entities: QueryEntities,
  resourceTree: ResourceTree<EntityRef>
): TreeTableItem => {
  if ('ResourceGroup' in resourceTree) {
    const node = resourceTree.ResourceGroup;
    const [entityType, entityId] = Object.entries(node.id)[0] as [EntityRefKey, string];
    const entity = lookupEntity(entities, entityType, entityId);
    const children = node.children.map(child => transformResourceTree(entities, child));
    const availableResourceTypes = collectResourceTypesFromTree(children);

    return {
      id: entityId,
      type: entityType,
      entity: entity as EntityTypeValue,
      icon: getIconForType(entityType),
      children,
      availableResourceTypes,
    };
  }

  const [entityType, entityId] = Object.entries(resourceTree.Resource)[0] as [EntityRefKey, string];
  const entity = lookupEntity(entities, entityType, entityId);

  return {
    id: entityId,
    type: entityType,
    entity: entity as EntityTypeValue,
    icon: getIconForType(entityType),
    children: [],
    availableResourceTypes: undefined,
  };
};

/** Recursively find a TreeTableItem by id */
export function findItemById(root: TreeTableItem, id: string): TreeTableItem | undefined {
  if (root.id === id) return root;
  if (root.children) {
    for (const child of root.children) {
      const found = findItemById(child, id);
      if (found) return found;
    }
  }
  return undefined;
}

/** Look up the FSM type name for a leaf resource from the query entities.
 *  If exactly 1 FSM uses this resource type, return that FSM name.
 *  If >1 FSM types use it, return null (all FSMs). */
function lookupFsmTypeName(item: TreeTableItem, entities: QueryEntities): string | null {
  const typeName =
    item.entity && 'type_name' in item.entity ? (item.entity.type_name as string) : undefined;
  const usedBy = typeName ? entities.resource_types[typeName]?.used_by : undefined;
  if (usedBy && usedBy.length === 1) return usedBy[0]!;
  return null;
}

/** Build TimelineRequest params for a single tree item.
 *  @param groupFsmFilters — per-item FSM filter for resource groups.
 *    Map value: null = aggregate all FSMs, string = filter to that FSM type.
 *    Missing key = fall back to first `used_by` entry (single-FSM) or null (multi-FSM).
 */
export function buildBulkParamsForItem(
  item: TreeTableItem,
  selectedTypes: Map<string, string>,
  entities: QueryEntities,
  config: TimelineConfig,
  groupFsmFilters?: Map<string, string | null>,
  operatorId: string | null = null
): TimelineRequest<TaskFilter> {
  const isGroup = item.type !== EntityTypeKey.Resource;
  const resourceTypeName = isGroup
    ? selectedTypes.get(item.id) || item.availableResourceTypes?.[0] || ''
    : undefined;
  const usedBy = resourceTypeName ? entities.resource_types[resourceTypeName]?.used_by : undefined;
  let fsmTypeName: string | null;
  if (usedBy?.length === 1) {
    fsmTypeName = usedBy[0]!;
  } else if (isGroup) {
    fsmTypeName = groupFsmFilters?.has(item.id) ? (groupFsmFilters.get(item.id) ?? null) : null;
  } else {
    fsmTypeName = lookupFsmTypeName(item, entities);
  }
  const threshold = getLongEntitiesThreshold(config.end - config.start);

  if (isGroup) {
    return {
      ResourceGroup: {
        resource_group_id: item.id,
        resource_type_name: resourceTypeName || '',
        long_entities_threshold_s: null,
        entity_filter: { entity_type_name: fsmTypeName },
        app_params: { operator_id: operatorId },
        config,
      },
    };
  }

  return {
    Resource: {
      resource_id: item.id,
      long_entities_threshold_s: threshold,
      entity_filter: { entity_type_name: fsmTypeName },
      application: { operator_id: operatorId },
      config,
    },
  };
}

/**
 * Collect all visible rows and their bulk request params.
 * A row is visible if it's the root or all of its ancestors are expanded.
 */
export function collectVisibleEntries(
  items: TreeTableItem[],
  expandedIds: Set<string>,
  selectedTypes: Map<string, string>,
  entities: QueryEntities,
  config: TimelineConfig,
  groupFsmFilters?: Map<string, string | null>,
  operatorId: string | null = null
): Record<string, TimelineRequest<TaskFilter>> {
  const result: Record<string, TimelineRequest<TaskFilter>> = {};

  function walk(item: TreeTableItem) {
    result[item.id] = buildBulkParamsForItem(
      item,
      selectedTypes,
      entities,
      config,
      groupFsmFilters,
      operatorId
    );

    if (item.children && expandedIds.has(item.id)) {
      for (const child of item.children) {
        walk(child);
      }
    }
  }

  for (const item of items) {
    walk(item);
  }
  return result;
}
