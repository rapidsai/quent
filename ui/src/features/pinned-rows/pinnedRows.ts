// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

interface TreeNode<T> {
  id: string;
  children?: readonly T[];
}

// Visit every row in tree order (parents before children), whether or not it is expanded.
function walkRows<T extends TreeNode<T>>(roots: readonly T[], visit: (row: T) => void) {
  for (const row of roots) {
    visit(row);
    if (row.children) {
      walkRows(row.children, visit);
    }
  }
}

export interface PinnedRowsResult<T> {
  // Pinned rows that are in the displayed tree, in the order the user arranged them.
  shownItems: T[];
  // Pinned rows that exist but a filter has removed from the displayed tree.
  hiddenCount: number;
}

/**
 * Work out which pinned rows to show.
 *
 * `fullRoots` is the whole tree, `displayedRoots` is the tree after filtering. Pins stay
 * stored even when a filter hides them, so clearing the filter brings them back. Ids that
 * are not in the full tree at all (for example, from an old shared link) are ignored.
 */
export function resolvePinnedRows<T extends TreeNode<T>>(
  pinnedIds: ReadonlySet<string>,
  fullRoots: readonly T[],
  displayedRoots: readonly T[]
): PinnedRowsResult<T> {
  if (pinnedIds.size === 0) {
    return { shownItems: [], hiddenCount: 0 };
  }

  const shownById = new Map<string, T>();
  walkRows(displayedRoots, row => {
    if (pinnedIds.has(row.id) && !shownById.has(row.id)) {
      shownById.set(row.id, row);
    }
  });
  // The set remembers the order pins were added or dragged into.
  const shownItems: T[] = [];
  for (const id of pinnedIds) {
    const row = shownById.get(id);
    if (row) {
      shownItems.push(row);
    }
  }

  let hiddenCount = 0;
  const countedIds = new Set<string>();
  walkRows(fullRoots, row => {
    if (pinnedIds.has(row.id) && !shownById.has(row.id) && !countedIds.has(row.id)) {
      countedIds.add(row.id);
      hiddenCount += 1;
    }
  });

  return { shownItems, hiddenCount };
}

/**
 * Move a pinned row to the position of the row it was dropped on, returning a new set. Pins
 * that a filter hides stay in the set, so they keep their place when the filter clears. If
 * either id is not pinned, or they are the same, the original set comes back unchanged.
 */
export function reorderPinnedIds(
  pinnedIds: Set<string>,
  activeId: string,
  overId: string
): Set<string> {
  if (activeId === overId || !pinnedIds.has(activeId) || !pinnedIds.has(overId)) {
    return pinnedIds;
  }
  const order = [...pinnedIds];
  const from = order.indexOf(activeId);
  const to = order.indexOf(overId);
  order.splice(from, 1);
  order.splice(to, 0, activeId);
  return new Set(order);
}

export interface PinAllPlan {
  // Matches to pin now, in match order. Already pinned rows are left out.
  toPin: string[];
}

/**
 * Work out what "pin all matches" should do. Matches keep their order, and duplicates and rows
 * that are already pinned are skipped.
 */
export function computePinAllPlan(
  matchIds: readonly string[],
  pinnedIds: ReadonlySet<string>
): PinAllPlan {
  const toPin = [...new Set(matchIds)].filter(id => !pinnedIds.has(id));

  return { toPin };
}

/**
 * Names of the rows above `targetId`, outermost first, or null if the row is not in the tree.
 * With a single root, the root is left out; with more than one root, each root is kept so
 * the path tells them apart.
 */
export function findAncestorNames<T extends TreeNode<T>>(
  roots: readonly T[],
  targetId: string,
  getName: (row: T) => string
): string[] | null {
  const keepRoots = roots.length > 1;

  const search = (row: T, above: readonly string[], isRoot: boolean): string[] | null => {
    if (row.id === targetId) {
      return [...above];
    }
    const next = isRoot && !keepRoots ? above : [...above, getName(row)];
    for (const child of row.children ?? []) {
      const found = search(child, next, false);
      if (found) {
        return found;
      }
    }
    return null;
  };

  for (const root of roots) {
    const found = search(root, [], true);
    if (found) {
      return found;
    }
  }
  return null;
}
