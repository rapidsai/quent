import { TimelineSeries } from '@/components/timeline/types';
import { formatBytes } from '@/services/formatters';
import { ResourceTimelineBinned } from '~quent/types/ResourceTimelineBinned';
import { ResourceTimelineBinnedByState } from '~quent/types/ResourceTimelineBinnedByState';

/** Type guard to check if data is ResourceTimelineBinnedByState */
function isBinnedByState(
  data: ResourceTimelineBinnedByState | ResourceTimelineBinned
): data is ResourceTimelineBinnedByState {
  return 'capacities_states_values' in data;
}

export function buildBinnedTimelineSeries(
  data: ResourceTimelineBinnedByState | ResourceTimelineBinned,
  startTime: bigint
): {
  timestamps: number[];
  series: TimelineSeries;
} {
  const { config } = data;
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

  if (isBinnedByState(data)) {
    const { capacities_states_values } = data;
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
  } else {
    // ResourceTimelineBinned: capacities_values (flat: capacity → values)
    const { capacities_values } = data;
    for (const [capacity, values] of Object.entries(capacities_values)) {
      const formatter = getFormatterForCapacityType(capacity);
      if (values) {
        series[capacity] = { formatter, values, binDuration: bin_duration };
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
