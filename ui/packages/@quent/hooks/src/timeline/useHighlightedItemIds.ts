// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo } from 'react';
import { useAtomValue } from 'jotai';
import { hoveredWorkerIdAtom } from '../atoms/dag';
import { operatorTimelineRowId } from '@quent/utils';

/**
 * Minimal tree node interface for useHighlightedItemIds.
 * App code can pass TreeTableItem directly — structural typing ensures compatibility.
 */
interface TreeNode {
  id: string;
  children?: TreeNode[];
}

/**
 * Returns the set of item IDs in the subtree rooted at the currently
 * hovered worker node (plus the synthetic Operator timeline row under that worker),
 * or undefined when nothing is hovered.
 */
export function useHighlightedItemIds(rootItem: TreeNode): Set<string> | undefined {
  const hoveredWorkerId = useAtomValue(hoveredWorkerIdAtom);

  return useMemo(() => {
    if (!hoveredWorkerId) return undefined;

    const ids = new Set<string>();

    function collectSubtree(items: TreeNode[]) {
      for (const item of items) {
        ids.add(item.id);
        if (item.children) collectSubtree(item.children);
      }
    }

    function find(items: TreeNode[]): boolean {
      for (const item of items) {
        if (item.id === hoveredWorkerId) {
          collectSubtree([item]);
          ids.add(operatorTimelineRowId(hoveredWorkerId));
          return true;
        }
        if (item.children && find(item.children)) return true;
      }
      return false;
    }

    find([rootItem]);
    return ids.size > 0 ? ids : undefined;
  }, [hoveredWorkerId, rootItem]);
}
