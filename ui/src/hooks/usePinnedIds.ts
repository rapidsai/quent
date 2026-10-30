// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback } from 'react';
import { useAtomValueRawSync, useSetAtom } from 'jotai';
import { pinnedCollapsedAtom, pinnedIdsAtom } from '@/atoms/resourceTree';
import { computePinAllPlan, reorderPinnedIds } from '@/features/pinned-rows/pinnedRows';

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
        return new Set(prev).add(id);
      });
    },
    [setPinnedIds]
  );

  // Pin several rows at once, after the existing pins.
  const pinMany = useCallback(
    (ids: readonly string[]) => {
      setPinnedIds(prev => {
        const { toPin } = computePinAllPlan(ids, prev);
        return toPin.length === 0 ? prev : new Set([...prev, ...toPin]);
      });
    },
    [setPinnedIds]
  );

  const unpinMany = useCallback(
    (ids: readonly string[]) => {
      setPinnedIds(prev => {
        const removing = new Set(ids);
        if (![...prev].some(id => removing.has(id))) {
          return prev;
        }
        return new Set([...prev].filter(id => !removing.has(id)));
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
    pinMany,
    unpinMany,
    movePin,
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
