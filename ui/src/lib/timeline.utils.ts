import { ResourceTimelineBinnedByState } from '~quent/types/ResourceTimelineBinnedByState';

export function buildBinnedTimelineSeries(data: ResourceTimelineBinnedByState): {
  timestamps: number[];
  series: Record<string, number[]>;
} {
  const { config, capacities_states_values } = data;
  const { span, bin_duration, num_bins } = config;

  // Generate timestamps from span.start, incrementing by bin_duration
  const timestamps: number[] = [];
  const numBinsNumber = Number(num_bins);
  for (let i = 0; i < numBinsNumber; i++) {
    const timestamp: bigint = span.start + BigInt(i) * BigInt(bin_duration);
    // Convert from nanoseconds to milliseconds for JS Date compatibility
    timestamps.push(Number(timestamp / 1_000_000n));
  }

  // Build series for each capacity key + state provided
  const series: Record<string, number[]> = {};
  const capacityTypes = Object.keys(capacities_states_values);
  capacityTypes.forEach(capacityType => {
    const capacityStateValues = capacities_states_values[capacityType] ?? {};
    for (const [key, values] of Object.entries(capacityStateValues)) {
      if (values) {
        series[key] = values;
      }
    }
  });

  return { timestamps, series };
}
