// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { TREE_TABLE_SCROLLER_SELECTOR, type PanelHeights } from '@quent/components';

// The pinned panel opens tall enough for this many rows. The user can drag it to show more.
const INITIAL_VISIBLE_PINNED_ROWS = 6;

const sum = (heights: readonly number[]) => heights.reduce((total, height) => total + height, 0);

/**
 * Reads the heights of the pinned panel from the page. The table sits in a scroller, so the
 * panel's own height says nothing about it; the scroller's first child is as tall as all of
 * the content. `header` is the part of the panel that sits outside the table.
 */
export function measurePinnedPanelHeights(
  panelContent: HTMLElement | null,
  header: HTMLElement | null
): PanelHeights | undefined {
  const table = panelContent?.querySelector(TREE_TABLE_SCROLLER_SELECTOR)?.firstElementChild;
  if (!panelContent || !(table instanceof HTMLElement)) {
    return undefined;
  }
  const content = table.offsetHeight + (header?.offsetHeight ?? 0);
  const rowHeights = Array.from(
    panelContent.querySelectorAll<HTMLElement>('[role="listitem"]'),
    row => row.offsetHeight
  );
  // Everything that is not a row: the headers and the spacing around the rows.
  const chrome = Math.max(content - sum(rowHeights), 0);
  return { content, fit: chrome + sum(rowHeights.slice(0, INITIAL_VISIBLE_PINNED_ROWS)) };
}
