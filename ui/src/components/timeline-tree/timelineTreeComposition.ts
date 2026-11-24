// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo } from 'react';
import { resolvePinnedRows } from '@/features/pinned-rows/pinnedRows';
import { usePinnedCollapsed, usePinnedIds } from '@/hooks/usePinnedIds';
import type { PinningControls, TimelineTreeItem } from './TimelineTreeTable';

export interface TimelineTreeFilterSummary {
  isActive: boolean;
  matchCount: number;
  pinnableMatchIds: readonly string[];
}

export interface TimelineTreePinningSource {
  fullRoots: readonly TimelineTreeItem[];
  displayedRoots: readonly TimelineTreeItem[];
  isPinnable?: (item: TimelineTreeItem) => boolean;
  preparePinnedItem?: (item: TimelineTreeItem) => TimelineTreeItem;
}

export function combineTimelineTreeFilters(
  filters: readonly TimelineTreeFilterSummary[]
): TimelineTreeFilterSummary {
  return {
    isActive: filters.some(filter => filter.isActive),
    matchCount: filters.reduce((total, filter) => total + filter.matchCount, 0),
    pinnableMatchIds: filters.flatMap(filter => filter.pinnableMatchIds),
  };
}

export function resolveTimelineTreePinning(
  sources: readonly TimelineTreePinningSource[],
  pinnedIds: ReadonlySet<string>
) {
  const shownById = new Map<string, TimelineTreeItem>();
  let hiddenCount = 0;

  for (const source of sources) {
    const result = resolvePinnedRows(
      pinnedIds,
      source.fullRoots,
      source.displayedRoots,
      source.isPinnable
    );
    hiddenCount += result.hiddenCount;
    for (const item of result.shownItems) {
      shownById.set(item.id, source.preparePinnedItem?.(item) ?? item);
    }
  }

  return {
    shownItems: [...pinnedIds].flatMap(id => {
      const item = shownById.get(id);
      return item ? [item] : [];
    }),
    hiddenCount,
  };
}

export function useCombinedTimelineTreePinning(sources: readonly TimelineTreePinningSource[]) {
  const { pinnedIds, togglePin, pinMany, clearPins, movePin } = usePinnedIds();
  const { isPinnedCollapsed, togglePinnedCollapsed } = usePinnedCollapsed();
  const { shownItems, hiddenCount } = useMemo(
    () => resolveTimelineTreePinning(sources, pinnedIds),
    [pinnedIds, sources]
  );
  const pinning = useMemo<PinningControls>(
    () => ({
      items: shownItems,
      hiddenCount,
      ids: pinnedIds,
      onToggle: togglePin,
      isCollapsed: isPinnedCollapsed,
      onToggleCollapsed: togglePinnedCollapsed,
      onClear: clearPins,
      onReorder: movePin,
    }),
    [
      clearPins,
      hiddenCount,
      isPinnedCollapsed,
      movePin,
      pinnedIds,
      shownItems,
      togglePin,
      togglePinnedCollapsed,
    ]
  );

  return { pinning, pinMany } as const;
}
