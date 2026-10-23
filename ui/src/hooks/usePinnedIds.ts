// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback } from 'react';
import { useAtomValueRawSync, useSetAtom } from 'jotai';
import { pinnedCollapsedAtom, pinnedIdsAtom } from '@/atoms/resourceTree';
import { MAX_PINNED_ROWS, reorderPinnedIds } from '@/features/pinned-rows/pinnedRows';

/* getter/setter for the ids of pinned timeline rows */
export function usePinnedIds() {
  const pinnedIds = useAtomValueRawSync(pinnedIdsAtom);
  const setPinnedIds = useSetAtom(pinnedIdsAtom);

  const togglePin = useCallback(
    (id: string) => {
      setPinnedIds(prev => {
        if (prev.has(id)) {
          const next = new Set(prev);
          next.delete(id);
          return next;
        }
        // At the limit, ignore the request and keep the same set.
        if (prev.size >= MAX_PINNED_ROWS) {
          return prev;
        }
        return new Set(prev).add(id);
      });
    },
    [setPinnedIds]
  );

  // Drop a pinned row onto another one to take its place in the list.
  const movePin = useCallback(
    (activeId: string, overId: string) =>
      setPinnedIds(prev => reorderPinnedIds(prev, activeId, overId)),
    [setPinnedIds]
  );

  return {
    pinnedIds,
    togglePin,
    movePin,
    isAtPinLimit: pinnedIds.size >= MAX_PINNED_ROWS,
  } as const;
}

/* getter/setter for whether the pinned section is collapsed to just its header */
export function usePinnedCollapsed() {
  const isPinnedCollapsed = useAtomValueRawSync(pinnedCollapsedAtom);
  const setPinnedCollapsed = useSetAtom(pinnedCollapsedAtom);

  const togglePinnedCollapsed = useCallback(
    () => setPinnedCollapsed(previous => !previous),
    [setPinnedCollapsed]
  );

  return { isPinnedCollapsed, togglePinnedCollapsed } as const;
}
