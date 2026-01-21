import { useQuery } from '@tanstack/react-query';
import { DEFEAULT_STALE_TIME, fetchResourceTimelineAggregated } from '@/services/api';
import { Timeline } from './Timeline';
import { TimelineSkeleton } from './TimelineSkeleton';
import { useMemo } from 'react';
import { buildBinnedTimelineSeries } from '@/lib/timeline.utils';

type ResourceTimelineProps = {
  engineId: string;
  resourceId: string;
};

export function ResourceTimeline({ engineId, resourceId }: ResourceTimelineProps) {
  const { data, isLoading, error } = useQuery({
    queryKey: ['resourceTimeline', engineId, resourceId],
    queryFn: () => fetchResourceTimelineAggregated(engineId, resourceId, { num_bins: 100 }),
    staleTime: DEFEAULT_STALE_TIME,
  });

  const { timestamps, series } = useMemo(() => {
    return data ? buildBinnedTimelineSeries(data) : { timestamps: [], series: {} };
  }, [data]);

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
  return <Timeline series={series} timestamps={timestamps ?? []} />;
}
