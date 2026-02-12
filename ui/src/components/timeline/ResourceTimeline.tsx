import { useQuery } from '@tanstack/react-query';
import {
  DEFAULT_STALE_TIME,
  fetchResourceTimeline,
  fetchResourceGroupTimeline,
} from '@/services/api';
import { useDeferredReady } from '@/hooks/useDeferredReady';
import { TimelineSkeleton } from './TimelineSkeleton';
import { useMemo, lazy, Suspense } from 'react';
import { buildBinnedTimelineSeries } from '@/lib/timeline.utils';
import { TimelineSeries } from './types';
import { EntityTypeKey } from '@/types';

// Lazy load Timeline to split echarts into a separate chunk
const Timeline = lazy(() => import('./Timeline').then(mod => ({ default: mod.Timeline })));

type ResourceTimelineProps = {
  engineId: string;
  queryId: string;
  resourceId: string;
  resourceType: string;
  startTime: bigint;
  durationSeconds: number;
  fsmTypeName?: string | undefined;
  resourceTypeName?: string;
  instanceName?: string;
  showTooltip?: boolean;
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
  resourceType,
  startTime,
  durationSeconds,
  fsmTypeName,
  resourceTypeName,
  instanceName,
  showTooltip = true,
}: ResourceTimelineProps) {
  const deferredReady = useDeferredReady();
  const queryFunction =
    resourceType === EntityTypeKey.ResourceGroup
      ? fetchResourceGroupTimeline
      : fetchResourceTimeline;
  const { data, isLoading, error } = useQuery({
    queryKey: ['resourceTimeline', engineId, queryId, resourceId, fsmTypeName, resourceTypeName],
    queryFn: () =>
      queryFunction(engineId, queryId, resourceId, {
        num_bins: 200,
        start: 0,
        end: durationSeconds,
        ...(fsmTypeName && { fsm_type_name: fsmTypeName }),
        ...(resourceTypeName && { resource_type_name: resourceTypeName }),
      }),
    staleTime: DEFAULT_STALE_TIME,
    enabled: deferredReady,
  });

  const { timestamps, series } = useMemo(() => {
    return data
      ? buildBinnedTimelineSeries(data, startTime)
      : { timestamps: [], series: EMPTY_TIMELINE_SERIES };
  }, [data, startTime]);

  if (!deferredReady || isLoading) {
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
      <Timeline
        series={series}
        timestamps={timestamps ?? []}
        startTime={startTime}
        colorKey={resourceType === EntityTypeKey.ResourceGroup ? instanceName : undefined}
        showTooltip={showTooltip}
      />
    </Suspense>
  );
}
