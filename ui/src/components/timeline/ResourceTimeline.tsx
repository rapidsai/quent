import { useQuery } from '@tanstack/react-query';
import {
  DEFAULT_STALE_TIME,
  fetchResourceTimeline,
  fetchResourceGroupTimeline,
} from '@/services/api';
import { useAtomValue } from 'jotai';
import { bulkInitializedAtom } from '@/atoms/timeline';
import { useDeferredReady } from '@/hooks/useDeferredReady';
import { TimelineSkeleton } from './TimelineSkeleton';
import { useMemo, lazy, Suspense } from 'react';
import { buildBinnedTimelineSeries, getAdaptiveNumBins } from '@/lib/timeline.utils';
import { TimelineSeries } from './types';
import { EntityTypeKey } from '@/types';
import { WHITE, withOpacity } from '@/services/colors';
import type { TimelineResponse } from '~quent/types/TimelineResponse';
import type { XAxisRange } from './Timeline';

const Timeline = lazy(() => import('./Timeline').then(mod => ({ default: mod.Timeline })));

type ResourceTimelineProps = {
  engineId: string;
  queryId: string;
  resourceId: string;
  resourceType: string;
  startTime: bigint;
  /** Total query duration in seconds — required by the timeline API */
  durationSeconds: number;
  fsmTypeName?: string | undefined;
  resourceTypeName?: string;
  instanceName?: string;
  showTooltip?: boolean;
  /** Pre-fetched timeline data from bulk endpoint; skips individual fetch when present */
  preloadedData?: TimelineResponse;
  /** When set, constrains the xAxis to this window (server-side zoom) */
  xAxisRange?: XAxisRange;
};

const EMPTY_TIMELINE_SERIES: TimelineSeries = {
  empty: {
    color: withOpacity(WHITE, 0),
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
  showTooltip = true,
  preloadedData,
  xAxisRange,
}: ResourceTimelineProps) {
  const deferredReady = useDeferredReady();
  const bulkInitialized = useAtomValue(bulkInitializedAtom);

  const queryFunction =
    resourceType === EntityTypeKey.ResourceGroup
      ? fetchResourceGroupTimeline
      : fetchResourceTimeline;
  const {
    data: fetchedData,
    isLoading,
    error,
  } = useQuery({
    queryKey: ['resourceTimeline', engineId, queryId, resourceId, fsmTypeName, resourceTypeName],
    queryFn: () =>
      queryFunction(engineId, queryId, resourceId, {
        num_bins: getAdaptiveNumBins(durationSeconds),
        start: 0,
        end: durationSeconds,
        duration: durationSeconds,
        ...(fsmTypeName && { fsm_type_name: fsmTypeName }),
        ...(resourceTypeName && { resource_type_name: resourceTypeName }),
      }),
    staleTime: DEFAULT_STALE_TIME,
    enabled: deferredReady && bulkInitialized && !preloadedData,
  });

  const data = preloadedData ?? fetchedData;

  const { timestamps, series } = useMemo(() => {
    return data
      ? buildBinnedTimelineSeries(data, startTime)
      : { timestamps: [], series: EMPTY_TIMELINE_SERIES };
  }, [data, startTime]);

  if (!preloadedData && (!deferredReady || isLoading)) {
    return <TimelineSkeleton />;
  }

  if (error) {
    return (
      <div className="flex items-center justify-center p-8 text-red-400">
        Failed to load timeline
      </div>
    );
  }

  return (
    <Suspense fallback={<TimelineSkeleton />}>
      <Timeline
        series={series}
        timestamps={timestamps ?? []}
        startTime={startTime}
        showTooltip={showTooltip}
        xAxisRange={xAxisRange}
      />
    </Suspense>
  );
}
