// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useMemo } from 'react';
import { useInfiniteEntityList } from '@quent/client';
import { useDebouncedZoomRange, useSelectedNodeIds } from '@quent/hooks';
import type { FsmTypeDecl, FiniteStateMachine } from '@quent/utils';
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
  fsmStateScope = 'all',
  onEntitySelect,
}: LongEntitiesRowProps) {
  const selectedNodeIds = useSelectedNodeIds();
  const debouncedZoomRange = useDebouncedZoomRange();
  const operatorIds = useMemo(() => [...selectedNodeIds], [selectedNodeIds]);
  const window =
    debouncedZoomRange.end > debouncedZoomRange.start
      ? debouncedZoomRange
      : { start: 0, end: durationSeconds };
  const minUsageSeconds = getLongEntitiesThreshold(window.end - window.start);

  const { data, fetchNextPage, hasNextPage, isFetching, isPlaceholderData } = useInfiniteEntityList(
    {
      engineId,
      queryId,
      window,
      operatorIds,
      minUsageSeconds,
      sortDir: 'Desc',
      maxItems: ENTITIES_PER_PAGE,
      filter: { scope: { Resource: { resource_id: resourceId } } },
    }
  );

  const entities = useMemo(() => data?.pages.flatMap(page => page.items) ?? [], [data]);
  const entries = useMemo(
    () =>
      buildLongEntityEntries(
        entities.map(e => e.entity),
        fsmTypes,
        isDark ? 'dark' : 'light',
        fsmStateScope === 'resource' ? new Set([resourceId]) : null
      ),
    [entities, fsmStateScope, fsmTypes, isDark, resourceId]
  );
  const totalEntities = data?.pages[data.pages.length - 1]?.total ?? entities.length;

  const handleEntityClick = useCallback(
    (entry: LongEntityEntry) => {
      if (!onEntitySelect) return;
      const fsm = entities.find(e => e.entity.id === entry.entityId)?.entity;
      if (fsm) onEntitySelect(fsm);
    },
    [entities, onEntitySelect]
  );

  if (!data && isFetching) {
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
    <div>
      <LongEntitiesGantt
        entries={entries}
        durationSeconds={durationSeconds}
        height={LONG_ENTITIES_TIMELINE_HEIGHT}
        isDark={isDark}
        onEntityClick={onEntitySelect ? handleEntityClick : undefined}
      />

      {hasNextPage && !isPlaceholderData && (
        <div className="flex justify-center border-t border-border/50 py-1">
          <Button
            type="button"
            variant="ghost"
            size="xs"
            className="h-4 px-1 text-[10px]"
            disabled={isFetching}
            onClick={event => {
              event.stopPropagation();
              void fetchNextPage();
            }}
          >
            Show more ({entities.length} of {totalEntities})
          </Button>
        </div>
      )}
    </div>
  );
}
