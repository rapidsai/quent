// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { DEFAULT_STALE_TIME, fetchSingleTimeline } from '@quent/client';
import {
  useBulkInitialized,
  useDebouncedZoomRange,
  timelineCacheKey,
  useTimelineData,
  useSelectedNodeIds,
  useSelectedOperatorLabel,
  useDeferredReady,
  useSetTimelineHover,
} from '@quent/hooks';
import { TimelineSkeleton } from './TimelineSkeleton';
import { TimelineTooltipPortal } from './TimelineTooltipPortal';
import { PlayheadLine } from './PlayheadLine';
import { resolveOverlayData, type RetainedOverlayData } from './resourceTimeline.utils';
import type { TimelineHoverPosition } from './Timeline';
import { useCallback, useEffect, useId, useMemo, useRef, useState, lazy, Suspense } from 'react';
import type { EChartsInstance } from 'echarts-for-react';
import {
  buildBinnedTimelineSeries,
  dimSeries,
  mergeOverlaySeries,
  getAdaptiveNumBins,
  getTimelineConfig,
} from '../lib/timeline.utils';
import { TimelineSeries } from './types';
import { EntityTypeKey } from '@quent/utils';
import { WHITE, withOpacity, type PaletteTheme } from '@quent/utils';
import type {
  SingleTimelineResponse,
  SingleTimelineRequest,
  QueryFilter,
  OperatorFilter,
  QuantitySpec,
  FsmTypeDecl,
  ResourceTypeDecl,
} from '@quent/utils';
const Timeline = lazy(() => import('./Timeline').then(mod => ({ default: mod.Timeline })));

type ResourceTimelineProps = {
  engineId: string;
  queryId: string;
  resourceId: string;
  resourceType: string;
  durationSeconds: number;
  fsmTypeName?: string | undefined;
  resourceTypeName?: string;
  instanceName?: string;
  showTooltip?: boolean;
  /** Pre-fetched timeline data from bulk endpoint; skips individual fetch when present */
  preloadedData?: SingleTimelineResponse;
  resourceTypeDecl?: ResourceTypeDecl;
  quantitySpecs?: { [key in string]?: QuantitySpec };
  fsmTypes?: { [key in string]?: FsmTypeDecl };
  /** Whether dark mode is active. Passed explicitly to decouple from ThemeContext. */
  isDark: boolean;
};

const EMPTY_TIMELINE_SERIES: TimelineSeries = {
  empty: {
    color: withOpacity(WHITE, 0),
    binDuration: 0,
    values: [],
    formatter: (value: number) => `${value}`,
  },
};

/** Per-resource timeline with automatic data fetching, zoom sync, and operator overlay. */
export function ResourceTimeline({
  engineId,
  queryId,
  resourceId,
  resourceType,
  durationSeconds,
  fsmTypeName,
  resourceTypeName,
  showTooltip = true,
  resourceTypeDecl,
  quantitySpecs,
  fsmTypes,
  isDark,
}: ResourceTimelineProps) {
  const paletteTheme: PaletteTheme = isDark ? 'dark' : 'light';
  const deferredReady = useDeferredReady();
  const zoomRange = useDebouncedZoomRange();
  const bulkInitialized = useBulkInitialized();
  const operatorLabel = useSelectedOperatorLabel();

  const selectedNodeIds = useSelectedNodeIds();
  const operatorIds = useMemo(() => [...selectedNodeIds].sort(), [selectedNodeIds]);
  const hasOperatorFilter = operatorIds.length > 0;

  const cacheResourceTypeName =
    resourceType === EntityTypeKey.ResourceGroup ? (resourceTypeName ?? '') : '';
  const baseCacheKey = timelineCacheKey({
    resourceId,
    resourceTypeName: cacheResourceTypeName,
    fsmTypeName,
  });
  const preloadedData = useTimelineData(baseCacheKey);

  const operatorCacheKey = timelineCacheKey({
    resourceId,
    resourceTypeName: cacheResourceTypeName,
    fsmTypeName,
    operatorIds,
  });
  const operatorTimelineData = useTimelineData(operatorCacheKey);
  // Retain overlay data for the same operator set while its atom is reseeded.
  const lastOverlayRef = useRef<RetainedOverlayData | null>(null);
  if (operatorTimelineData !== undefined) {
    lastOverlayRef.current = { cacheKey: operatorCacheKey, data: operatorTimelineData };
  } else if (!hasOperatorFilter) {
    lastOverlayRef.current = null;
  }
  const overlayPreloadedData = resolveOverlayData(
    operatorTimelineData,
    lastOverlayRef.current,
    operatorCacheKey,
    hasOperatorFilter
  );

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
      const config = {
        num_bins: getAdaptiveNumBins(),
        start,
        end,
      };
      const request: SingleTimelineRequest<QueryFilter, OperatorFilter> = {
        entry: isGroup
          ? {
              ResourceGroup: {
                resource_group_id: resourceId,
                resource_type_name: resourceTypeName ?? '',
                long_entities_threshold_s: null,
                entity_filter: { entity_type_name: fsmTypeName ?? null },
                app_params: { operator_ids: [] },
                config,
              },
            }
          : {
              Resource: {
                resource_id: resourceId,
                long_entities_threshold_s: null,
                entity_filter: { entity_type_name: fsmTypeName ?? null },
                application: { operator_ids: [] },
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

  const { timestamps, series, yAxisLabel } = useMemo<{
    timestamps: number[];
    series: TimelineSeries;
    yAxisLabel?: string;
  }>(() => {
    const data = preloadedData ?? fetchedData;
    if (!data) {
      return { timestamps: [], series: EMPTY_TIMELINE_SERIES };
    }

    const base = buildBinnedTimelineSeries(
      data.data,
      data.config,
      paletteTheme,
      resourceTypeDecl,
      quantitySpecs,
      fsmTypes
    );

    if (hasOperatorFilter && operatorLabel) {
      if (overlayPreloadedData) {
        const baseSpan = getTimelineConfig(data).span;
        const opSpan = getTimelineConfig(overlayPreloadedData).span;
        const baseEqualsOpsSpan = baseSpan.start === opSpan.start && baseSpan.end === opSpan.end;
        if (baseEqualsOpsSpan) {
          const opResult = buildBinnedTimelineSeries(
            overlayPreloadedData.data,
            overlayPreloadedData.config,
            paletteTheme,
            resourceTypeDecl,
            quantitySpecs,
            fsmTypes
          );
          return {
            timestamps: base.timestamps,
            series: mergeOverlaySeries(base.series, opResult.series, operatorLabel),
            yAxisLabel: base.yAxisLabel,
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
        yAxisLabel: base.yAxisLabel,
      };
    }

    return base;
  }, [
    preloadedData,
    fetchedData,
    hasOperatorFilter,
    overlayPreloadedData,
    resourceTypeDecl,
    quantitySpecs,
    fsmTypes,
    operatorLabel,
    paletteTheme,
  ]);

  // Bridge the chart's atom-unaware `onHoverChange` callback into the shared
  // `timelineHoverAtom` that the global tooltip portal subscribes to. The
  // stable per-row `ownerId` (from React's `useId`) tags writes so cleanups
  // (pointerleave, drag start, unmount) only clear the atom when *this* row
  // is the current owner — preventing a stale leave from clobbering a fresh
  // enter on a neighbouring row during fast pointer transitions.
  //
  // Declared before any conditional `return` so hook order stays stable
  // across the loading / error / data render branches.
  const ownerId = useId();
  const setTimelineHover = useSetTimelineHover();
  const [chartInstance, setChartInstance] = useState<EChartsInstance | null>(null);
  const handleChartReady = useCallback((instance: EChartsInstance) => {
    setChartInstance(instance);
  }, []);
  const handleHoverChange = useCallback(
    (position: TimelineHoverPosition | null) => {
      if (position == null) {
        setTimelineHover(prev => (prev?.sourceId === ownerId ? null : prev));
      } else {
        setTimelineHover({ ...position, sourceId: ownerId });
      }
    },
    [ownerId, setTimelineHover]
  );
  useEffect(() => {
    return () => {
      setTimelineHover(prev => (prev?.sourceId === ownerId ? null : prev));
    };
  }, [ownerId, setTimelineHover]);

  if (!preloadedData && (!deferredReady || isLoading)) {
    return <TimelineSkeleton />;
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-full text-red-400 text-xs">
        Failed to load timeline
      </div>
    );
  }

  const effectiveYAxisLabel = yAxisLabel ?? fsmTypeName;

  return (
    <div className="relative h-full w-full">
      <Suspense fallback={<TimelineSkeleton />}>
        <Timeline
          series={series}
          timestamps={timestamps ?? []}
          durationSeconds={durationSeconds}
          showTooltip={showTooltip}
          isDark={isDark}
          yAxisLabel={effectiveYAxisLabel}
          onHoverChange={handleHoverChange}
          onReady={handleChartReady}
        />
        {showTooltip && (
          <TimelineTooltipPortal ownerId={ownerId} series={series} timestamps={timestamps ?? []} />
        )}
      </Suspense>
      <PlayheadLine instance={chartInstance} />
    </div>
  );
}
