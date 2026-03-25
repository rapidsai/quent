// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useState, useCallback } from 'react';

/* getter/setter for tracking expanded IDs in the resource tree */
export function useExpandedIds(initialId?: string) {
  const [expandedIds, setExpandedIds] = useState<Set<string>>(() => {
    return initialId ? new Set([initialId]) : new Set();
  });

  const handleExpandChange = useCallback((itemId: string, isExpanded: boolean) => {
    setExpandedIds(prev => {
      const next = new Set(prev);
      if (isExpanded) {
        next.add(itemId);
      } else {
        next.delete(itemId);
      }
      return next;
    });
  }, []);

  return { expandedIds, handleExpandChange } as const;
}
