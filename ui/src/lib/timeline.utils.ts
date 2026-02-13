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

export function buildBinnedTimelineSeries(
  data: TimelineResponse,
  startTime: bigint
): {
  timestamps: number[];
  series: TimelineSeries;
} {
  const config = 'Binned' in data ? data.Binned.config : data.BinnedByState.config;
  const { bin_duration, num_bins } = config;

  // Generate timestamps from span.start, incrementing by bin_duration
  const timestamps: number[] = [];
  const numBinsNumber = Number(num_bins);
  const startTimeMillis = Number(startTime / 1_000_000n);
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
      if (values) {
        series[capacity] = { formatter, values, binDuration: bin_duration };
      }
    }
  } else if ('BinnedByState' in data) {
    const { capacities_states_values } = data.BinnedByState;
    for (const capacityType of Object.keys(capacities_states_values)) {
      const capacityStateValues = capacities_states_values[capacityType] ?? {};
      for (const [state, values] of Object.entries(capacityStateValues)) {
        const formatter = getFormatterForCapacityType(capacityType);
        if (values) {
          series[state] = {
            binDuration: bin_duration,
            formatter,
            values,
          };
        }
      }
    }
  }

  return { timestamps, series };
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

export const connectChart = (instance: EChartsInstance, chartGroup: string = CHART_GROUP) => {
  // Sync zoom state from any existing chart in the group before connecting
  const existingInstance = findExistingChartInGroup(chartGroup);
  if (existingInstance) {
    const existingOption = existingInstance.getOption();
    const dataZoomOption = existingOption.dataZoom as Array<{ start?: number; end?: number }>;

    if (dataZoomOption && dataZoomOption[0]) {
      const { start, end } = dataZoomOption[0];
      if (start !== undefined && end !== undefined) {
        instance.setOption({
          dataZoom: [{ start, end }],
        });
      }
    }
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
