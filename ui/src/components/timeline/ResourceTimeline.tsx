import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { DEFAULT_STALE_TIME, fetchSingleTimeline } from '@/services/api';
import { useAtomValue } from 'jotai';
import { bulkInitializedAtom, debouncedZoomRangeAtom } from '@/atoms/timeline';
import { useDeferredReady } from '@/hooks/useDeferredReady';
import { TimelineSkeleton } from './TimelineSkeleton';
import { useMemo, lazy, Suspense } from 'react';
import { buildBinnedTimelineSeries, getAdaptiveNumBins } from '@/lib/timeline.utils';
import { TimelineSeries } from './types';
import { EntityTypeKey } from '@/types';
import { WHITE, withOpacity } from '@/services/colors';
import type { SingleTimelineResponse } from '~quent/types/SingleTimelineResponse';
import type { SingleTimelineRequest } from '~quent/types/SingleTimelineRequest';
import type { QueryFilter } from '~quent/types/QueryFilter';
import type { TaskFilter } from '~quent/types/TaskFilter';
import type { XAxisRange } from './Timeline';
import type { ZoomRange } from './TimelineController';

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
  preloadedData?: SingleTimelineResponse;
  /** When set, fetches only this time window instead of the full duration */
  zoomRange?: ZoomRange;
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
  const zoomRange = useAtomValue(debouncedZoomRangeAtom);
  const bulkInitialized = useAtomValue(bulkInitializedAtom);

  const {
    data: fetchedData,
    isLoading,
    error,
  } = useQuery({
    queryKey: [
      'singleTimeline',
      engineId,
      queryId,
      resourceId,
      fsmTypeName,
      resourceTypeName,
      zoomRange,
    ],
    queryFn: () => {
      const isGroup = resourceType === EntityTypeKey.ResourceGroup;
      const start = zoomRange?.start ?? 0;
      const end = zoomRange?.end ?? durationSeconds;
      const request: SingleTimelineRequest<QueryFilter, TaskFilter> = {
        config: {
          num_bins: getAdaptiveNumBins(end - start),
          start,
          end,
        },
        entry: isGroup
          ? {
              ResourceGroup: {
                resource_group_id: resourceId,
                resource_type_name: resourceTypeName ?? '',
                long_entities_threshold_s: null,
                entity_filter: { entity_type_name: fsmTypeName ?? null },
                app_params: { operator_id: null },
              },
            }
          : {
              Resource: {
                resource_id: resourceId,
                long_entities_threshold_s: null,
                entity_filter: { entity_type_name: fsmTypeName ?? null },
                application: { operator_id: null },
              },
            },
        app_params: { query_id: queryId },
      };
      return fetchSingleTimeline(engineId, request, durationSeconds);
    },
    staleTime: DEFAULT_STALE_TIME,
    enabled: deferredReady && !preloadedData && bulkInitialized,
    placeholderData: keepPreviousData,
  });

  const { timestamps, series } = useMemo(() => {
    const response = preloadedData ?? fetchedData;
    if (!response) return { timestamps: [], series: EMPTY_TIMELINE_SERIES };
    return buildBinnedTimelineSeries(response.data, response.config, startTime);
  }, [preloadedData, fetchedData, startTime]);

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
