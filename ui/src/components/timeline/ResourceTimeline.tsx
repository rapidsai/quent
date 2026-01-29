import { useQuery } from '@tanstack/react-query';
import { DEFAULT_STALE_TIME, fetchResourceTimeline } from '@/services/api';
import { TimelineSkeleton } from './TimelineSkeleton';
import { useMemo, lazy, Suspense } from 'react';
import { buildBinnedTimelineSeries } from '@/lib/timeline.utils';
import { TimelineSeries } from './types';

// Lazy load Timeline to split echarts into a separate chunk
const Timeline = lazy(() => import('./Timeline').then(mod => ({ default: mod.Timeline })));

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
    queryFn: () =>
      fetchResourceTimeline(engineId, queryId, resourceId, {
        num_bins: 100,
        start: 0,
        end: durationSeconds,
        ...(fsmTypeName && { fsm_type_name: fsmTypeName }),
      }),
    staleTime: DEFAULT_STALE_TIME,
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
  return (
    <Suspense fallback={<TimelineSkeleton />}>
      <Timeline series={series} timestamps={timestamps ?? []} startTime={startTime} />
    </Suspense>
  );
}
