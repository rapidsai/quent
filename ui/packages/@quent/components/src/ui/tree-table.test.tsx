// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { act, fireEvent, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { TREE_TABLE_SCROLLER_SELECTOR, TreeTable, type Column } from './tree-table';

interface Row {
  id: string;
  name: string;
  children?: Row[];
}

const ROW_HEIGHT = 40;
const row = (id: string): Row => ({ id, name: `Row ${id}` });

const renderCalls: { id: string; isPinned: boolean | undefined }[] = [];
const columns: Column<Row>[] = [
  {
    key: 'name',
    label: 'Name',
    widthIndex: 0,
    isFirst: true,
    render: ({ item, isPinned }) => {
      renderCalls.push({ id: item.id, isPinned });
      return <span>{item.name}</span>;
    },
  },
];

interface PinnedProps {
  onReorderPinned?: (activeId: string, overId: string) => void;
  pinnedItems?: Row[];
  hiddenPinnedCount?: number;
  isPinnedCollapsed?: boolean;
  onTogglePinnedCollapsed?: () => void;
}

function renderTable(pinned: PinnedProps = {}, tableColumns: Column<Row>[] = columns) {
  return render(
    <TreeTable<Row>
      data={[row('a'), row('b')]}
      columns={tableColumns}
      virtualized
      rowHeight={ROW_HEIGHT}
      {...pinned}
    />
  );
}

describe('TreeTable pinned section', () => {
  it('renders nothing extra when no rows are pinned', () => {
    renderTable();

    expect(screen.queryByRole('region', { name: 'Pinned rows' })).not.toBeInTheDocument();
  });

  it('renders each pinned row through the columns with isPinned set', () => {
    renderCalls.length = 0;
    renderTable({ pinnedItems: [row('x'), row('y')] });

    const region = screen.getByRole('region', { name: 'Pinned rows' });
    expect(within(region).getByText('Row x')).toBeInTheDocument();
    expect(within(region).getByText('Row y')).toBeInTheDocument();
    // The table re-renders while it measures itself, so compare the distinct rows drawn as pinned.
    const pinnedIds = new Set(renderCalls.filter(call => call.isPinned).map(call => call.id));
    expect([...pinnedIds]).toEqual(['x', 'y']);
    expect(renderCalls.some(call => call.id === 'a' && call.isPinned)).toBe(false);
  });

  it('shows how many rows are pinned in the header', () => {
    renderTable({ pinnedItems: [row('x'), row('y'), row('z')] });

    expect(screen.getByRole('button', { name: /Pinned \(3\)/ })).toHaveAttribute(
      'aria-expanded',
      'true'
    );
  });

  it('hides the rows but keeps the header when collapsed', () => {
    renderTable({ pinnedItems: [row('x')], isPinnedCollapsed: true });

    expect(screen.getByRole('button', { name: /Pinned \(1\)/ })).toHaveAttribute(
      'aria-expanded',
      'false'
    );
    expect(screen.queryByText('Row x')).not.toBeInTheDocument();
  });

  it('asks to toggle collapsing when the header is clicked', async () => {
    const onTogglePinnedCollapsed = vi.fn();
    renderTable({ pinnedItems: [row('x')], onTogglePinnedCollapsed });

    await userEvent.click(screen.getByRole('button', { name: /Pinned \(1\)/ }));

    expect(onTogglePinnedCollapsed).toHaveBeenCalledTimes(1);
  });

  it('explains pins that a filter has hidden', () => {
    renderTable({ pinnedItems: [row('x')], hiddenPinnedCount: 2 });

    expect(screen.getByText('2 hidden by filter')).toBeInTheDocument();
  });

  it('keeps the header when every pin is hidden by a filter', () => {
    renderTable({ pinnedItems: [], hiddenPinnedCount: 2 });

    expect(screen.getByRole('button', { name: /Pinned \(0\)/ })).toBeInTheDocument();
    expect(screen.getByText('2 hidden by filter')).toBeInTheDocument();
  });

  it('shows every pinned row, with no height cap', () => {
    const pinnedItems = Array.from({ length: 8 }, (_, i) => row(`p${i}`));
    renderTable({ pinnedItems });

    const list = screen.getByRole('list', { name: 'Pinned rows list' });
    expect(list.style.maxHeight).toBe('');
    expect(within(list).getAllByRole('listitem')).toHaveLength(8);
  });

  it('pads the bottom of the pinned section so the last row does not sit on the divider', () => {
    renderTable({ pinnedItems: [row('x')] });

    expect(screen.getByRole('region', { name: 'Pinned rows' })).toHaveClass('pb-4');
  });

  it('sits inside the sticky header so it stays put while the tree scrolls', () => {
    renderTable({ pinnedItems: [row('x')] });

    const region = screen.getByRole('region', { name: 'Pinned rows' });
    expect(region.closest<HTMLElement>('[style*="sticky"]')).not.toBeNull();
  });
});

describe('TreeTable pinned row reordering', () => {
  const controlColumns: Column<Row>[] = [
    {
      key: 'name',
      label: 'Name',
      widthIndex: 0,
      isFirst: true,
      render: ({ item }) => (
        <>
          <span>{item.name}</span>
          <button type="button" onClick={() => onPinClick(item.id)}>
            Pin {item.name}
          </button>
        </>
      ),
    },
  ];
  const onPinClick = vi.fn();
  const ROW_STEP = 40;
  let restoreRect: () => void;

  // jsdom has no layout, so give each pinned row its own slot for the drag to measure.
  beforeEach(() => {
    onPinClick.mockClear();
    const original = Element.prototype.getBoundingClientRect;
    Element.prototype.getBoundingClientRect = function rect(this: Element) {
      const index =
        this.getAttribute('role') === 'listitem'
          ? Array.from(this.parentElement?.children ?? []).indexOf(this)
          : 0;
      const top = index * ROW_STEP;
      // The list is as tall as all of its rows, so rows can be dragged anywhere inside it.
      const isList = this.getAttribute('role') === 'list';
      const height = isList ? this.children.length * ROW_STEP : ROW_STEP;
      return {
        x: 0,
        y: top,
        top,
        left: 0,
        width: 200,
        height,
        right: 200,
        bottom: top + height,
        toJSON: () => ({}),
      };
    };
    restoreRect = () => {
      Element.prototype.getBoundingClientRect = original;
    };
  });
  afterEach(() => restoreRect());

  // The drag measures the rows after it starts, so give it a moment before the next key.
  const settle = () => act(() => new Promise(resolve => setTimeout(resolve, 50)));

  const pinnedRow = (name: string) =>
    within(screen.getByRole('region', { name: 'Pinned rows' }))
      .getByText(name)
      .closest('[role="listitem"]') as HTMLElement;

  // A second column stands in for the timelines.
  const withTimeline: Column<Row>[] = [
    ...controlColumns,
    {
      key: 'usage',
      label: 'Usage',
      widthIndex: 1,
      render: ({ item }) => <span>Chart {item.id}</span>,
    },
  ];

  // The info cell (name and type) is the only part of a row that picks it up.
  const infoCellOf = (name: string) =>
    within(pinnedRow(name)).getByText(name).closest('.group\\/info') as HTMLElement;

  it('reports the row that was dropped and the row it took the place of', async () => {
    const onReorderPinned = vi.fn();
    renderTable({ pinnedItems: [row('x'), row('y'), row('z')], onReorderPinned }, withTimeline);

    infoCellOf('Row x').focus();
    await userEvent.keyboard(' ');
    await settle();
    await userEvent.keyboard('{ArrowDown}');
    await settle();
    await userEvent.keyboard(' ');

    expect(onReorderPinned).toHaveBeenCalledTimes(1);
    expect(onReorderPinned).toHaveBeenCalledWith('x', 'y');
  });

  // Press, move past the drag distance, then release, the way a mouse drag arrives.
  const dragWithPointer = async (from: HTMLElement, deltaY: number) => {
    const press = { pointerId: 1, isPrimary: true, button: 0, clientX: 10, clientY: 10 };
    fireEvent.pointerDown(from, press);
    await settle();
    fireEvent.pointerMove(document, { ...press, clientY: 10 + deltaY / 2 });
    fireEvent.pointerMove(document, { ...press, clientY: 10 + deltaY });
    await settle();
    fireEvent.pointerUp(document, { ...press, clientY: 10 + deltaY });
    await settle();
  };

  it('starts a drag with the pointer from anywhere in the info cell', async () => {
    const onReorderPinned = vi.fn();
    renderTable({ pinnedItems: [row('x'), row('y')], onReorderPinned }, withTimeline);

    await dragWithPointer(within(pinnedRow('Row x')).getByText('Row x'), ROW_STEP);

    expect(onReorderPinned).toHaveBeenCalledWith('x', 'y');
  });

  it('does not start a drag with the pointer from a control inside the info cell', async () => {
    const onReorderPinned = vi.fn();
    renderTable({ pinnedItems: [row('x'), row('y')], onReorderPinned }, withTimeline);

    await dragWithPointer(
      within(pinnedRow('Row x')).getByRole('button', { name: 'Pin Row x' }),
      ROW_STEP
    );

    expect(onReorderPinned).not.toHaveBeenCalled();
  });

  it('does not start a drag with the pointer from the timeline', async () => {
    const onReorderPinned = vi.fn();
    renderTable({ pinnedItems: [row('x'), row('y')], onReorderPinned }, withTimeline);

    await dragWithPointer(within(pinnedRow('Row x')).getByText('Chart x'), ROW_STEP);

    expect(onReorderPinned).not.toHaveBeenCalled();
  });
});

describe('TreeTable info cell', () => {
  it('is a named hover group so controls in it can ignore hovering the other columns', () => {
    renderTable({ pinnedItems: [row('x')] });

    const cell = within(screen.getByRole('region', { name: 'Pinned rows' }))
      .getByText('Row x')
      .closest('.group\\/info');

    expect(cell).not.toBeNull();
  });
});

describe('TreeTable scroller', () => {
  it('marks its scroll container so callers can find it', () => {
    const { container } = renderTable();

    expect(container.querySelectorAll(TREE_TABLE_SCROLLER_SELECTOR)).toHaveLength(1);
  });
});
