import { useQuery } from '@tanstack/react-query';
import {
  DEFEAULT_STALE_TIME,
  fetchResourceTimelineAggregated,
  fetchResourceTimelineAggregatedByFSM,
} from '@/services/api';
import { Timeline } from './Timeline';
import { TimelineSkeleton } from './TimelineSkeleton';
import { useMemo } from 'react';
import { buildBinnedTimelineSeries } from '@/lib/timeline.utils';
import { ResourceTimelineBinnedByState } from '~quent/types/ResourceTimelineBinnedByState';
import { ResourceTimelineBinned } from '~quent/types/ResourceTimelineBinned';
import { TimelineSeries } from './types';

type ResourceTimelineProps = {
  engineId: string;
  queryId: string;
  resourceId: string;
  startTime: bigint;
  durationSeconds: number;
  fsmTypeName: string | undefined;
};

const EMPTY_TIMELINE_SERIES: TimelineSeries = {
  empty: {
    binDuration: 0,
    values: [],
    formatter: (value: number) => `${value}`,
  },
};

export function ResourceTimeline({
  engineId,
  queryId,
  resourceId,
  startTime,
  durationSeconds,
  fsmTypeName,
}: ResourceTimelineProps) {
  const { data, isLoading, error } = useQuery({
    queryKey: ['resourceTimeline', engineId, queryId, resourceId, fsmTypeName],
    // TODO (joe): Dynamic number of bins
    queryFn: (): Promise<ResourceTimelineBinnedByState | ResourceTimelineBinned> =>
      fsmTypeName
        ? fetchResourceTimelineAggregatedByFSM(engineId, queryId, resourceId, {
            num_bins: 100,
            start: 0,
            end: durationSeconds,
            fsm_type_name: fsmTypeName,
          })
        : fetchResourceTimelineAggregated(engineId, queryId, resourceId, {
            num_bins: 100,
            start: 0,
            end: durationSeconds,
          }),
    staleTime: DEFEAULT_STALE_TIME,
  });

  const { timestamps, series } = useMemo(() => {
    return data
      ? buildBinnedTimelineSeries(data, startTime)
      : { timestamps: [], series: EMPTY_TIMELINE_SERIES };
  }, [data, startTime]);

  if (isLoading) {
    return <TimelineSkeleton />;
  }

  if (error) {
    return (
      <div className="flex items-center justify-center p-8 text-red-400">
        Failed to load timeline
      </div>
    );
  }

  // This should render a timeline one way or another
  return <Timeline series={series} timestamps={timestamps ?? []} startTime={startTime} />;
}
