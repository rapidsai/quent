import { TimelineSeries } from '@/components/timeline/types';
import { TreeTableItem } from '@/components/resource-tree/types';
import { formatBytes } from '@/services/formatters';
import { entityRefToEntitiesKey } from '@/lib/queryBundle.utils';
import { collectResourceTypesFromTree, getIconForType } from '@/lib/resource.utils';
import { TimelineResponse } from '~quent/types/TimelineResponse';
import { QueryEntities } from '~quent/types/QueryEntities';
import { ResourceTree } from '~quent/types/ResourceTree';
import { EntityTypeValue, EntityRefKey } from '@/types';
import type { EChartsInstance } from 'echarts-for-react';
import { connect, getInstanceByDom } from '@/lib/echarts';
import { CHART_GROUP } from '@/components/timeline/Timeline';
import { getColorForKey, WHITE, withOpacity } from '@/services/colors';

export function buildBinnedTimelineSeries(
  data: TimelineResponse,
  startTime: bigint
): {
  timestamps: number[];
  series: TimelineSeries;
} {
  const config = 'Binned' in data ? data.Binned.config : data.BinnedByState.config;
  const { bin_duration, num_bins, span } = config;

  // Generate timestamps from span.start, incrementing by bin_duration
  const timestamps: number[] = [];
  const numBinsNumber = Number(num_bins);
  const startTimeMillis = Number(startTime / 1_000_000n) + span.start * 1_000;
  for (let i = 0; i < numBinsNumber; i++) {
    const timestampMillis: number = startTimeMillis + i * bin_duration * 1_000;
    // Convert from nanoseconds to milliseconds for JS Date compatibility
    timestamps.push(Math.round(timestampMillis));
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

export const connectChart = (instance: EChartsInstance, chartGroup: string = CHART_GROUP) => {
  // Sync zoom state from any existing chart in the group before connecting
  const zoomState = getChartGroupZoomState(chartGroup);
  if (zoomState) {
    instance.setOption({
      dataZoom: [zoomState],
    });
  }

  // Activate the dataZoom brush tool by default
  instance.dispatchAction({
    type: 'takeGlobalCursor',
    key: 'dataZoomSelect',
    dataZoomSelectActive: true,
  });

  instance.group = chartGroup;
  connect(chartGroup);
};

// Helper function to lookup entity from QueryEntities
const lookupEntity = (
  entities: QueryEntities,
  entityType: EntityRefKey,
  entityId: string
): EntityTypeValue | undefined => {
  const entityKey = entityRefToEntitiesKey(entityType) as keyof QueryEntities;
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
  resourceTree: ResourceTree
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
