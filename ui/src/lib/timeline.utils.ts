import { TimelineSeries, TimelineMark } from '@/components/timeline/types';
import { TreeTableItem } from '@/components/resource-tree/types';
import { formatBytes } from '@/services/formatters';
import { entityRefToEntitiesKey } from '@/lib/queryBundle.utils';
import { collectResourceTypesFromTree, getIconForType } from '@/lib/resource.utils';
import type { ResourceTimeline } from '~quent/types/ResourceTimeline';
import { QueryEntities } from '~quent/types/QueryEntities';
import { ResourceTree } from '~quent/types/ResourceTree';
import type { EntityRef } from '~quent/types/EntityRef';
import { EntityTypeValue, EntityRefKey, EntityTypeKey } from '@/types';
import type { EChartsInstance } from 'echarts-for-react';
import { connect, getInstanceByDom } from '@/lib/echarts';
import { CHART_GROUP } from '@/components/timeline/Timeline';
import { getColorForKey, lightenColor, WHITE, withOpacity } from '@/services/colors';
import type { BinnedSpanSec } from '~quent/types/BinnedSpanSec';
import type { SingleTimelineResponse } from '~quent/types/SingleTimelineResponse';
import type { FiniteStateMachine } from '~quent/types/FiniteStateMachine';
import type { TimelineRequest } from '~quent/types/TimelineRequest';
import type { TaskFilter } from '~quent/types/TaskFilter';

const MAX_TIMELINE_BINS = 400;
const LONG_ENTITIES_BIN_MULTIPLIER = 30;

/**
 * Computes the number of bins such that each bin is >= 1ms wide.
 * For a 50ms window this returns 50; for windows >= 200ms it returns 200.
 */
export function getAdaptiveNumBins(windowSeconds: number): number {
  const windowMs = windowSeconds * 1_000;
  return Math.max(1, Math.min(MAX_TIMELINE_BINS, Math.round(windowMs)));
}

/** Threshold for "long" entities: 10x the current bin duration in seconds. */
export function getLongEntitiesThreshold(windowSeconds: number): number {
  const numBins = getAdaptiveNumBins(windowSeconds);
  return LONG_ENTITIES_BIN_MULTIPLIER * (windowSeconds / numBins);
}

export function buildBinnedTimelineSeries(
  data: ResourceTimeline,
  config: BinnedSpanSec,
  startTime: bigint
): {
  timestamps: number[];
  series: TimelineSeries;
} {
  const { bin_duration, num_bins, span } = config;

  const timestamps: number[] = [];
  const numBinsNumber = Number(num_bins);
  const startTimeMillis = Number(startTime / 1_000_000n) + span.start * 1_000;
  for (let i = 0; i < numBinsNumber; i++) {
    timestamps.push(Math.round(startTimeMillis + i * bin_duration * 1_000));
  }

  // Build series based on data type
  const series: TimelineSeries = {};

  if ('Binned' in data) {
    // ResourceTimelineBinned: capacities_values (flat: capacity → values)
    const { capacities_values } = data.Binned;
    for (const [capacity, values] of Object.entries(capacities_values)) {
      const formatter = getFormatterForCapacityType(capacity);
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
        const formatter = getFormatterForCapacityType(capacityType);
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
 */
export function buildTimelineMarks(
  longFsms: FiniteStateMachine[],
  startTime: bigint
): TimelineMark[] | undefined {
  if (longFsms.length === 0) return undefined;

  const startTimeMs = Number(startTime / 1_000_000n);

  const marks = longFsms.flatMap(fsm => {
    const label = fsm.instance_name || fsm.id;
    return fsm.transitions
      .slice(0, -1)
      .map((transition, i) => {
        const next = fsm.transitions[i + 1];
        const xStart = Math.round(startTimeMs + transition.timestamp * 1000);
        const xEnd = Math.round(startTimeMs + next.timestamp * 1000);
        return { label, stateName: transition.name, xStart, xEnd };
      })
      .filter(m => m.xEnd > m.xStart);
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

/** Clone entries and set operator_id on each TimelineRequest */
export function setOperatorOnEntries(
  baseEntries: Record<string, TimelineRequest<TaskFilter>>,
  operatorId: string
): Record<string, TimelineRequest<TaskFilter>> {
  const result: Record<string, TimelineRequest<TaskFilter>> = {};
  for (const [id, entry] of Object.entries(baseEntries)) {
    if ('ResourceGroup' in entry) {
      result[id] = {
        ResourceGroup: {
          ...entry.ResourceGroup,
          app_params: { ...entry.ResourceGroup.app_params, operator_id: operatorId },
        },
      };
    } else {
      result[id] = {
        Resource: {
          ...entry.Resource,
          application: { ...entry.Resource.application, operator_id: operatorId },
        },
      };
    }
  }
  return result;
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

function getFormatterForCapacityType(capacityType: string): (value: number) => string {
  switch (capacityType) {
    case 'bytes':
      return (value: number) => formatBytes(value, 0);
    default:
      return (value: number) => String(value);
  }
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
  // Sync zoom state from any existing chart in the group before connecting
  const zoomState = getChartGroupZoomState(chartGroup);
  if (zoomState) {
    instance.setOption({
      dataZoom: [zoomState],
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

/** Look up the FSM type name for a tree item from the query entities */
function lookupFsmTypeName(item: TreeTableItem, entities: QueryEntities): string | null {
  const entity = item.entity;
  const entityTypeName = 'type_name' in entity ? (entity.type_name as string) : undefined;
  const usedBy = entityTypeName ? entities.resource_types[entityTypeName]?.used_by : undefined;
  return usedBy?.[0] ?? null;
}

/** Build TimelineRequest params for a single tree item */
export function buildBulkParamsForItem(
  item: TreeTableItem,
  selectedTypes: Map<string, string>,
  entities: QueryEntities,
  operatorId: string | null = null,
  windowSeconds?: number
): TimelineRequest<TaskFilter> {
  const fsmTypeName = lookupFsmTypeName(item, entities);
  const isGroup = item.type !== EntityTypeKey.Resource;
  const threshold = windowSeconds != null ? getLongEntitiesThreshold(windowSeconds) : null;

  if (isGroup) {
    const resourceTypeName = selectedTypes.get(item.id) || item.availableResourceTypes?.[0] || '';
    return {
      ResourceGroup: {
        resource_group_id: item.id,
        resource_type_name: resourceTypeName,
        long_entities_threshold_s: threshold,
        entity_filter: { entity_type_name: fsmTypeName },
        app_params: { operator_id: operatorId },
      },
    };
  }

  return {
    Resource: {
      resource_id: item.id,
      long_entities_threshold_s: threshold,
      entity_filter: { entity_type_name: fsmTypeName },
      application: { operator_id: operatorId },
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
  operatorId: string | null = null,
  windowSeconds?: number
): Record<string, TimelineRequest<TaskFilter>> {
  const result: Record<string, TimelineRequest<TaskFilter>> = {};

  function walk(item: TreeTableItem) {
    result[item.id] = buildBulkParamsForItem(
      item,
      selectedTypes,
      entities,
      operatorId,
      windowSeconds
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
