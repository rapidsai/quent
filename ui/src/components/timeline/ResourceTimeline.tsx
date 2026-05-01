// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { DEFAULT_STALE_TIME, fetchSingleTimeline } from '@/services/api';
import { useAtomValue } from 'jotai';
import {
  bulkInitializedAtom,
  debouncedZoomRangeAtom,
  hideTasksAtom,
  timelineCacheKey,
  timelineDataAtom,
} from '@/atoms/timeline';
import { selectedNodeIdsAtom, selectedOperatorLabelAtom } from '@/atoms/dag';
import { useDeferredReady } from '@/hooks/useDeferredReady';
import { TimelineSkeleton } from './TimelineSkeleton';
import { useMemo, useRef, lazy, Suspense } from 'react';
import {
  buildBinnedTimelineSeries,
  buildTimelineMarks,
  dimSeries,
  getLongFsms,
  mergeOverlaySeries,
  getAdaptiveNumBins,
  getTimelineConfig,
  getLongEntitiesThreshold,
} from '@/lib/timeline.utils';
import { TimelineSeries, TimelineMark } from './types';
import { EntityTypeKey } from '@/types';
import { WHITE, withOpacity } from '@/services/colors';
import { THEME_DARK, useTheme } from '@/contexts/ThemeContext';
import type { SingleTimelineResponse } from '~quent/types/SingleTimelineResponse';
import type { SingleTimelineRequest } from '~quent/types/SingleTimelineRequest';
import type { QueryFilter } from '~quent/types/QueryFilter';
import type { TaskFilter } from '~quent/types/TaskFilter';
import type { CapacityDecl } from '~quent/types/CapacityDecl';
import type { QuantitySpec } from '~quent/types/QuantitySpec';
import type { FsmTypeDecl } from '~quent/types/FsmTypeDecl';
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
  /** Pre-fetched timeline data from bulk endpoint; skips individual fetch when present */
  preloadedData?: SingleTimelineResponse;
  capacities?: CapacityDecl[];
  quantitySpecs?: { [key in string]?: QuantitySpec };
  fsmTypes?: { [key in string]?: FsmTypeDecl };
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
  capacities,
  quantitySpecs,
  fsmTypes,
}: ResourceTimelineProps) {
  const { theme } = useTheme();
  const paletteTheme = theme === THEME_DARK ? 'dark' : 'light';
  const deferredReady = useDeferredReady();
  const zoomRange = useAtomValue(debouncedZoomRangeAtom);
  const bulkInitialized = useAtomValue(bulkInitializedAtom);
  const operatorLabel = useAtomValue(selectedOperatorLabelAtom);
  const hideTasks = useAtomValue(hideTasksAtom);

  const selectedNodeIds = useAtomValue(selectedNodeIdsAtom);
  const operatorId = selectedNodeIds.size > 0 ? selectedNodeIds.values().next().value! : null;

  const cacheResourceTypeName =
    resourceType === EntityTypeKey.ResourceGroup ? (resourceTypeName ?? '') : '';
  const baseCacheKey = timelineCacheKey({
    resourceId,
    resourceTypeName: cacheResourceTypeName,
    fsmTypeName,
  });
  const preloadedData = useAtomValue(timelineDataAtom(baseCacheKey));

  const operatorCacheKey = timelineCacheKey({
    resourceId,
    resourceTypeName: cacheResourceTypeName,
    fsmTypeName,
    operatorId,
  });
  const operatorTimelineData = useAtomValue(timelineDataAtom(operatorCacheKey));
  // Preserve the last non-undefined overlay data while an operator is selected.
  // Without this, switching operators causes a one-render undimmed flash because
  // the new operator's atom is empty until the seed effect fires.
  const lastOverlayRef = useRef<typeof operatorTimelineData>(undefined);
  if (operatorTimelineData !== undefined) {
    lastOverlayRef.current = operatorTimelineData;
  } else if (!operatorId) {
    lastOverlayRef.current = undefined;
  }
  const overlayPreloadedData = operatorId
    ? (operatorTimelineData ?? lastOverlayRef.current)
    : undefined;

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
      const windowSeconds = end - start;
      const config = {
        num_bins: getAdaptiveNumBins(),
        start,
        end,
      };
      const request: SingleTimelineRequest<QueryFilter, TaskFilter> = {
        entry: isGroup
          ? {
              ResourceGroup: {
                resource_group_id: resourceId,
                resource_type_name: resourceTypeName ?? '',
                long_entities_threshold_s: getLongEntitiesThreshold(windowSeconds),
                entity_filter: { entity_type_name: fsmTypeName ?? null },
                app_params: { operator_id: null },
                config,
              },
            }
          : {
              Resource: {
                resource_id: resourceId,
                long_entities_threshold_s: getLongEntitiesThreshold(windowSeconds),
                entity_filter: { entity_type_name: fsmTypeName ?? null },
                application: { operator_id: null },
                config,
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

  const { timestamps, series, marks } = useMemo<{
    timestamps: number[];
    series: TimelineSeries;
    marks?: TimelineMark[];
  }>(() => {
    const data = preloadedData ?? fetchedData;
    if (!data) return { timestamps: [], series: EMPTY_TIMELINE_SERIES };

    const base = buildBinnedTimelineSeries(
      data.data,
      data.config,
      startTime,
      paletteTheme,
      capacities,
      quantitySpecs,
      fsmTypes
    );
    const longFsms = getLongFsms(data.data);
    const filterSet =
      resourceType === EntityTypeKey.Resource ? new Set([resourceId]) : new Set<string>();

    const timelineMarks = buildTimelineMarks(longFsms, startTime, paletteTheme, filterSet, fsmTypes);

    if (operatorId && operatorLabel) {
      if (overlayPreloadedData) {
        const baseSpan = getTimelineConfig(data).span;
        const opSpan = getTimelineConfig(overlayPreloadedData).span;
        const baseEqualsOpsSpan = baseSpan.start === opSpan.start && baseSpan.end === opSpan.end;
        if (baseEqualsOpsSpan) {
          const opResult = buildBinnedTimelineSeries(
            overlayPreloadedData.data,
            overlayPreloadedData.config,
            startTime,
            paletteTheme,
            capacities,
            quantitySpecs,
            fsmTypes
          );
          const opLongFsmIds = new Set(getLongFsms(overlayPreloadedData.data).map(f => f.id));
          return {
            timestamps: base.timestamps,
            series: mergeOverlaySeries(base.series, opResult.series, operatorLabel),
            marks: buildTimelineMarks(
              longFsms,
              startTime,
              paletteTheme,
              filterSet,
              fsmTypes,
              opLongFsmIds,
              operatorLabel
            ),
          };
        }
      }
      // Operator is selected but the overlay can't render this frame
      // (data not yet populated for the new operator, or zoom span mismatch).
      // Dim the base anyway so the chart never flashes back to full color
      // between the click and the new overlay arriving.
      return {
        timestamps: base.timestamps,
        series: dimSeries(base.series),
        marks: timelineMarks,
      };
    }

    return { ...base, marks: timelineMarks };
  }, [
    preloadedData,
    fetchedData,
    operatorId,
    overlayPreloadedData,
    startTime,
    capacities,
    quantitySpecs,
    fsmTypes,
    resourceType,
    resourceId,
    operatorLabel,
    paletteTheme,
  ]);

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
        durationSeconds={durationSeconds}
        showTooltip={showTooltip}
        marks={hideTasks ? undefined : marks}
      />
    </Suspense>
  );
}
