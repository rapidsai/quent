// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useMemo } from 'react';
import { useAtomValueRawSync, useSetAtom } from 'jotai';
import { expandedIdsAtom, initializedDisclosureIdsAtom } from '@/atoms/resourceTree';

interface ExpandableTreeItem {
  id: string;
  children?: readonly ExpandableTreeItem[];
}

// Seed matching paths into user-controlled state so found rows appear without staying forced open.
export function useAutoExpandMatchingAncestors(
  root: ExpandableTreeItem | null,
  matchingIds: ReadonlySet<string>,
  enabled: boolean
) {
  const setExpandedIds = useSetAtom(expandedIdsAtom);
  const ancestorIds = useMemo(() => {
    const result = new Set<string>();
    if (!enabled || !root) {
      return result;
    }

    const visit = (item: ExpandableTreeItem): boolean => {
      const hasMatchingDescendant = item.children?.some(visit) ?? false;
      if (hasMatchingDescendant) {
        result.add(item.id);
      }
      return matchingIds.has(item.id) || hasMatchingDescendant;
    };
    visit(root);
    return result;
  }, [enabled, matchingIds, root]);

  useEffect(() => {
    if (ancestorIds.size === 0) {
      return;
    }
    setExpandedIds(previous => {
      const missingIds = [...ancestorIds].filter(id => !previous.has(id));
      return missingIds.length > 0 ? new Set([...previous, ...missingIds]) : previous;
    });
  }, [ancestorIds, setExpandedIds]);
}

/* getter/setter for tracking expanded IDs in the resource tree */
export function useExpandedIds(initialId?: string) {
  const expandedIds = useAtomValueRawSync(expandedIdsAtom);
  const setExpandedIds = useSetAtom(expandedIdsAtom);

  // Seed the initial id once without discarding expansion restored by other tree features.
  useEffect(() => {
    if (!initialId) {
      return;
    }
    setExpandedIds(prev => (prev.has(initialId) ? prev : new Set([...prev, initialId])));
  }, [initialId, setExpandedIds]);

  const handleExpandChange = useCallback(
    (itemId: string, isExpanded: boolean) => {
      setExpandedIds(prev => {
        const next = new Set(prev);
        if (isExpanded) {
          next.add(itemId);
        } else {
          next.delete(itemId);
        }
        return next;
      });
    },
    [setExpandedIds]
  );

  const handleManyExpandChange = useCallback(
    (itemIds: readonly string[], isExpanded: boolean) => {
      setExpandedIds(prev => {
        const next = new Set(prev);
        for (const itemId of itemIds) {
          if (isExpanded) {
            next.add(itemId);
          } else {
            next.delete(itemId);
          }
        }
        return next;
      });
    },
    [setExpandedIds]
  );

  return { expandedIds, handleExpandChange, handleManyExpandChange } as const;
}

/** Expands newly discovered disclosures once, while preserving later user choices. */
export function useInitializeDisclosureExpansion(
  disclosures: readonly { id: string; ownerId?: string }[]
) {
  const initializedIds = useAtomValueRawSync(initializedDisclosureIdsAtom);
  const setInitializedIds = useSetAtom(initializedDisclosureIdsAtom);
  const setExpandedIds = useSetAtom(expandedIdsAtom);

  useEffect(() => {
    const newDisclosures = disclosures.filter(disclosure => !initializedIds.has(disclosure.id));
    if (newDisclosures.length === 0) {
      return;
    }
    const newDisclosureIds = newDisclosures.map(disclosure => disclosure.id);
    const newOwnerIds = newDisclosures.flatMap(disclosure =>
      disclosure.ownerId ? [disclosure.ownerId] : []
    );
    setExpandedIds(previous => new Set([...previous, ...newOwnerIds, ...newDisclosureIds]));
    setInitializedIds(previous => new Set([...previous, ...newDisclosureIds]));
  }, [disclosures, initializedIds, setExpandedIds, setInitializedIds]);
}
