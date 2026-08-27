// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useMemo, useRef, useState } from 'react';
import { useEntityList } from '@quent/client';
import {
  useBulkInitialized,
  useDebouncedZoomRange,
  useLongEntityDensity,
  useReturnedTimelineIsStale,
  useReturnedTimelineNumBins,
  useSelectedNodeIds,
} from '@quent/hooks';
import { type FiniteStateMachine, type FsmTypeDecl, MAX_TIMELINE_BINS } from '@quent/utils';
import {
  Button,
  LONG_ENTITIES_TIMELINE_HEIGHT,
  LongEntitiesGantt,
  Skeleton,
  buildLongEntityEntries,
  getLongEntitiesThreshold,
  type LongEntityEntry,
} from '@quent/components';

const ENTITIES_PER_PAGE = 100;

type LongEntitiesRowProps = {
  engineId: string;
  queryId: string;
  /** The resource this row's entities are scoped to. */
  resourceId: string;
  durationSeconds: number;
  fsmTypes: { [key in string]?: FsmTypeDecl };
  isDark: boolean;
  /** Defaults to all states; resource scope keeps states used on this row's resource. */
  fsmStateScope?: 'all' | 'resource';
  onEntitySelect?: (fsm: FiniteStateMachine) => void;
  selectedEntityId?: string;
  onBackgroundClick?: () => void;
};

/**
 * Per-resource long-entities Gantt. Fetches the resource's entities (ranked by
 * longest usage) as soon as the row is shown and renders them as a compact
 * stacked Gantt directly under the timeline.
 */
export function LongEntitiesRow({
  engineId,
  queryId,
  resourceId,
  durationSeconds,
  fsmTypes,
  isDark,
  fsmStateScope = 'resource',
  onEntitySelect,
  selectedEntityId,
  onBackgroundClick,
}: LongEntitiesRowProps) {
  const selectedNodeIds = useSelectedNodeIds();
  const debouncedZoomRange = useDebouncedZoomRange();
  const bulkInitialized = useBulkInitialized();
  const longEntityDensity = useLongEntityDensity();
  const returnedNumBins = useReturnedTimelineNumBins(resourceId);
  const returnedTimelineIsStale = useReturnedTimelineIsStale(resourceId);
  const previousMinUsageSeconds = useRef<number | null>(null);
  const [maxEntities, setMaxEntities] = useState(ENTITIES_PER_PAGE);
  const operatorIds = useMemo(() => [...selectedNodeIds], [selectedNodeIds]);
  const zoomWindow =
    debouncedZoomRange.end > debouncedZoomRange.start
      ? debouncedZoomRange
      : { start: 0, end: durationSeconds };

  const defaultNumBins = MAX_TIMELINE_BINS * 2;
  const initializedAndNoBins = !returnedTimelineIsStale && bulkInitialized;
  const numBins = returnedNumBins ?? (initializedAndNoBins ? defaultNumBins : undefined);

  // Retain the rendered threshold while the next viewport loads.
  const minUsageSeconds =
    numBins == null
      ? null
      : getLongEntitiesThreshold(zoomWindow.end - zoomWindow.start, numBins, longEntityDensity);
  if (minUsageSeconds != null) {
    previousMinUsageSeconds.current = minUsageSeconds;
  }
  const displayedMinUsageSeconds = minUsageSeconds ?? previousMinUsageSeconds.current;

  const { data, isFetching, isPlaceholderData } = useEntityList(
    {
      engineId,
      queryId,
      window: zoomWindow,
      operatorIds,
      minUsageSeconds,
      sortDir: 'Desc',
      maxItems: maxEntities,
      filter: { scope: { Resource: { resource_id: resourceId } } },
    },
    { enabled: numBins != null }
  );

  const entities = useMemo(() => data?.items ?? [], [data]);
  const entries = useMemo(
    () =>
      buildLongEntityEntries(
        entities,
        fsmTypes,
        isDark ? 'dark' : 'light',
        fsmStateScope === 'resource' ? new Set([resourceId]) : null
      ),
    [entities, fsmStateScope, fsmTypes, isDark, resourceId]
  );
  const totalEntities = data?.total ?? entities.length;
  const hasMoreEntities = entities.length < totalEntities;
  const isLoadingMore = isPlaceholderData && entities.length < maxEntities;
  const showMoreButton = hasMoreEntities && (!isLoadingMore || maxEntities < totalEntities);

  const handleEntityClick = useCallback(
    (entry: LongEntityEntry) => {
      if (!onEntitySelect) {
        return;
      }
      const fsm = entities.find(e => e.id === entry.entityId);
      if (fsm) {
        onEntitySelect(fsm);
      }
    },
    [entities, onEntitySelect]
  );

  if (displayedMinUsageSeconds == null || (!data && isFetching)) {
    return (
      <div
        role="status"
        aria-label="Loading entities"
        className="flex flex-col justify-center gap-1.5 overflow-hidden px-2"
        style={{ height: LONG_ENTITIES_TIMELINE_HEIGHT }}
      >
        <Skeleton className="h-3 w-2/5" />
        <Skeleton className="ml-[18%] h-3 w-1/2" />
        <Skeleton className="ml-[55%] h-3 w-1/4" />
      </div>
    );
  }

  return (
    <div data-long-entities-gantt>
      <LongEntitiesGantt
        entries={entries}
        durationSeconds={durationSeconds}
        minUsageSeconds={displayedMinUsageSeconds}
        height={LONG_ENTITIES_TIMELINE_HEIGHT}
        isDark={isDark}
        onEntityClick={onEntitySelect ? handleEntityClick : undefined}
        selectedEntityId={selectedEntityId}
        onBackgroundClick={onBackgroundClick}
      />

      {showMoreButton && (
        <div className="flex justify-center border-t border-border/50 py-1">
          <Button
            type="button"
            variant="ghost"
            size="xs"
            className="h-4 px-1 text-[10px]"
            disabled={isFetching}
            onClick={event => {
              event.stopPropagation();
              setMaxEntities(current => current + ENTITIES_PER_PAGE);
            }}
          >
            {isLoadingMore ? 'Loading...' : `Show more (${entities.length} of ${totalEntities})`}
          </Button>
        </div>
      )}
    </div>
  );
}
