// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect } from 'react';
import { useAtom } from 'jotai';
import { expandedIdsAtom } from '@/atoms/resourceTree';

/* getter/setter for tracking expanded IDs in the resource tree */
export function useExpandedIds(initialId?: string) {
  const [expandedIds, setExpandedIds] = useAtom(expandedIdsAtom);

  // Seed with the initial id only when the atom is empty so that
  // navigating away and back keeps the user's expansion intact.
  useEffect(() => {
    if (!initialId) return;
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
