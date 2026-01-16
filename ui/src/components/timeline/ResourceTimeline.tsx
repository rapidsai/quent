import { useQuery } from '@tanstack/react-query';
import { DEFEAULT_STALE_TIME, fetchResourceTimeline, generateResourceUsage } from '@/services/api';
import { Timeline } from './Timeline';
import { TimelineSkeleton } from './TimelineSkeleton';
import { useMemo } from 'react';

type ResourceTimelineProps = {
  engineId: string;
  resourceId: string;
};

export function ResourceTimeline({ engineId, resourceId }: ResourceTimelineProps) {
  const { data, isLoading, error } = useQuery({
    queryKey: ['resourceTimeline', engineId, resourceId],
    queryFn: () => fetchResourceTimeline(engineId, resourceId),
    staleTime: DEFEAULT_STALE_TIME,
  });

  const [timestamps, series] = useMemo(() => {
    return data ? generateResourceUsage(data?.span) : [];
  }, [data]);
  const [, series2] = useMemo(() => {
    return data ? generateResourceUsage(data?.span) : [];
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
  return (
    <Timeline
      series={{ Usage: series ?? [], Usage2: series2 ?? [] }}
      timestamps={timestamps ?? []}
    />
  );
}
