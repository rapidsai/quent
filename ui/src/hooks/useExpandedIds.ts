// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useMemo } from 'react';
import { useAtom, useSetAtom } from 'jotai';
import { expandedIdsAtom } from '@/atoms/resourceTree';

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
  const [expandedIds, setExpandedIds] = useAtom(expandedIdsAtom);

  // Seed with the initial id only when the atom is empty so that
  // navigating away and back keeps the user's expansion intact.
  useEffect(() => {
    if (!initialId) {
      return;
    }
    setExpandedIds(prev => (prev.size === 0 ? new Set([initialId]) : prev));
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

  return { expandedIds, handleExpandChange } as const;
}
