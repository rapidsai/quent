// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render } from '@testing-library/react';
import { TreeTable, type Column } from '@quent/components';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { measurePinnedPanelHeights } from './pinnedPanelHeights';

const ROW_HEIGHT = 40;
const TABLE_PADDING = 16;
const HEADER_HEIGHT = 30;

function withHeight<T extends HTMLElement>(element: T, height: number): T {
  Object.defineProperty(element, 'offsetHeight', { value: height, configurable: true });
  return element;
}

// Mirrors the pinned table: a scroller whose first child holds the rows.
function buildPanel(rowCount: number, options: { header?: boolean; scroller?: boolean } = {}) {
  const { header = true, scroller = true } = options;
  const panel = document.createElement('div');
  const headerElement = withHeight(document.createElement('div'), HEADER_HEIGHT);
  if (header) {
    panel.append(headerElement);
  }

  const scrollerElement = document.createElement('div');
  if (scroller) {
    scrollerElement.setAttribute('data-tree-table-scroller', '');
  }
  const table = withHeight(document.createElement('div'), TABLE_PADDING + rowCount * ROW_HEIGHT);
  for (let index = 0; index < rowCount; index += 1) {
    const row = withHeight(document.createElement('div'), ROW_HEIGHT);
    row.setAttribute('role', 'listitem');
    table.append(row);
  }
  scrollerElement.append(table);
  panel.append(scrollerElement);

  return { panel, header: header ? headerElement : null };
}

describe('measurePinnedPanelHeights', () => {
  it('returns nothing when there is no panel', () => {
    expect(measurePinnedPanelHeights(null, null)).toBeUndefined();
  });

  it('returns nothing when the table scroller is not rendered', () => {
    const { panel, header } = buildPanel(3, { scroller: false });

    expect(measurePinnedPanelHeights(panel, header)).toBeUndefined();
  });

  it('measures the whole content including the header', () => {
    const { panel, header } = buildPanel(3);

    expect(measurePinnedPanelHeights(panel, header)?.content).toBe(
      HEADER_HEIGHT + TABLE_PADDING + 3 * ROW_HEIGHT
    );
  });

  it('fits the first six rows and everything that is not a row', () => {
    const { panel, header } = buildPanel(10);

    expect(measurePinnedPanelHeights(panel, header)?.fit).toBe(
      HEADER_HEIGHT + TABLE_PADDING + 6 * ROW_HEIGHT
    );
  });

  it('fits all the rows when there are fewer than six', () => {
    const { panel, header } = buildPanel(2);

    expect(measurePinnedPanelHeights(panel, header)).toEqual({
      content: HEADER_HEIGHT + TABLE_PADDING + 2 * ROW_HEIGHT,
      fit: HEADER_HEIGHT + TABLE_PADDING + 2 * ROW_HEIGHT,
    });
  });

  it('fits to the content when no rows are rendered, as when collapsed', () => {
    const { panel, header } = buildPanel(0);

    expect(measurePinnedPanelHeights(panel, header)).toEqual({
      content: HEADER_HEIGHT + TABLE_PADDING,
      fit: HEADER_HEIGHT + TABLE_PADDING,
    });
  });

  it('counts a missing header as no height', () => {
    const { panel, header } = buildPanel(2, { header: false });

    expect(measurePinnedPanelHeights(panel, header)).toEqual({
      content: TABLE_PADDING + 2 * ROW_HEIGHT,
      fit: TABLE_PADDING + 2 * ROW_HEIGHT,
    });
  });
});

interface Row {
  id: string;
  name: string;
}

const columns: Column<Row>[] = [
  {
    key: 'name',
    label: 'Name',
    widthIndex: 0,
    isFirst: true,
    render: ({ item }) => <span>{item.name}</span>,
  },
];

describe('measurePinnedPanelHeights on a real TreeTable', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  // Rows are 40 tall and everything else 400, so the numbers only come out right when both
  // the table content and every pinned row are found in the table's own markup.
  it('finds the table content and every pinned row', () => {
    vi.spyOn(HTMLElement.prototype, 'offsetHeight', 'get').mockImplementation(function (
      this: HTMLElement
    ) {
      return this.getAttribute('role') === 'listitem' ? ROW_HEIGHT : 400;
    });
    const pinnedItems: Row[] = Array.from({ length: 7 }, (_, index) => ({
      id: `row-${index}`,
      name: `Row ${index}`,
    }));

    const { container } = render(
      <TreeTable<Row> data={[]} columns={columns} pinnedItems={pinnedItems} hidePinnedToggle />
    );

    expect(measurePinnedPanelHeights(container, null)).toEqual({
      content: 400,
      fit: 400 - 7 * ROW_HEIGHT + 6 * ROW_HEIGHT,
    });
  });
});
