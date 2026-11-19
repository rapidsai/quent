// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { TREE_TABLE_SCROLLER_SELECTOR, type PanelHeights } from '@quent/components';

// The pinned panel opens tall enough for this many rows. The user can drag it to show more.
const INITIAL_VISIBLE_PINNED_ROWS = 6;

const sum = (heights: readonly number[]) => heights.reduce((total, height) => total + height, 0);

/**
 * Height of the first `limit` rows. A long list only has the rows near the viewport in the page,
 * and right after the panel appears it may have none, so any of the first rows that are not there
 * are guessed from the ones that are, or from the list's own row height estimate.
 */
function heightOfFirstRows(list: HTMLElement | null, rows: readonly HTMLElement[], limit: number) {
  const total = Number(list?.dataset.count ?? rows.length);
  const wanted = Math.min(limit, total);
  const first = rows.filter(row => Number(row.dataset.index) < wanted);
  const measured = sum(first.map(row => row.offsetHeight));
  const average =
    rows.length > 0
      ? sum(rows.map(row => row.offsetHeight)) / rows.length
      : Number(list?.dataset.rowHeight ?? 0);
  return measured + (wanted - first.length) * average;
}

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
  // The list is as tall as every pinned row, even the ones that are not in the page.
  const list = panelContent.querySelector<HTMLElement>('[role="list"]');
  const listHeight = list?.offsetHeight ?? 0;
  const rows = Array.from(panelContent.querySelectorAll<HTMLElement>('[role="listitem"]'));
  // Everything that is not a row: the headers and the spacing around the rows.
  const chrome = Math.max(content - listHeight, 0);
  return { content, fit: chrome + heightOfFirstRows(list, rows, INITIAL_VISIBLE_PINNED_ROWS) };
}
