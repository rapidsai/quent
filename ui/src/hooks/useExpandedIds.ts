// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useAtom } from 'jotai';
import { useCallback } from 'react';
import { expandedIdsAtom } from '@/atoms/resourceTree';

/**
 * Getter/setter for tracking expanded IDs in the resource tree.
 * Backed by expandedIdsAtom — initial state is set via useHydrateAtoms in QueryResourceTree.
 */
export function useExpandedIds() {
  const [expandedIds, setExpandedIds] = useAtom(expandedIdsAtom);

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
