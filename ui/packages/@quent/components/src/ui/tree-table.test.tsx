// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { act, fireEvent, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { TREE_TABLE_SCROLLER_SELECTOR, TreeTable, type Column } from './tree-table';

interface Row {
  id: string;
  name: string;
  rendersInParent?: boolean;
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
  onClearPinned?: () => void;
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

// jsdom has no layout, so the virtualizer would see a scroller with no height and draw no rows.
// Give the scroller a height and each row its minimum height, the way a browser would.
const SCROLLER_HEIGHT = 600;
const originalOffsetHeight = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'offsetHeight');
const originalClientHeight = Object.getOwnPropertyDescriptor(Element.prototype, 'clientHeight');

function mockLayout() {
  Object.defineProperty(Element.prototype, 'clientHeight', {
    configurable: true,
    get(this: Element) {
      return this.hasAttribute('data-tree-table-scroller') ? SCROLLER_HEIGHT : 0;
    },
  });
  Object.defineProperty(HTMLElement.prototype, 'offsetHeight', {
    configurable: true,
    get(this: HTMLElement) {
      return this.getAttribute('role') === 'listitem' ? ROW_HEIGHT : 0;
    },
  });
}

function restoreLayout() {
  if (originalOffsetHeight) {
    Object.defineProperty(HTMLElement.prototype, 'offsetHeight', originalOffsetHeight);
  }
  if (originalClientHeight) {
    Object.defineProperty(Element.prototype, 'clientHeight', originalClientHeight);
  }
}

describe('TreeTable rows drawn by their parent', () => {
  beforeEach(mockLayout);
  afterEach(restoreLayout);

  const parent: Row = {
    ...row('a'),
    children: [{ ...row('a-chart'), rendersInParent: true }, row('a-next')],
  };

  it('does not give an open parent a separate row for a child its parent draws', () => {
    render(
      <TreeTable<Row>
        data={[parent]}
        columns={columns}
        virtualized
        rowHeight={ROW_HEIGHT}
        controlledExpandedIds={new Set(['a'])}
      />
    );

    expect(screen.getByText('Row a')).toBeInTheDocument();
    expect(screen.getByText('Row a-next')).toBeInTheDocument();
    expect(screen.queryByText('Row a-chart')).not.toBeInTheDocument();
  });

  it('does not give a pinned parent a separate row for it either', () => {
    render(
      <TreeTable<Row>
        data={[]}
        columns={columns}
        virtualized
        rowHeight={ROW_HEIGHT}
        pinnedItems={[{ ...row('x'), children: [{ ...row('x-chart'), rendersInParent: true }] }]}
        controlledExpandedIds={new Set(['x'])}
      />
    );

    expect(screen.getByText('Row x')).toBeInTheDocument();
    expect(screen.queryByText('Row x-chart')).not.toBeInTheDocument();
  });
});

describe('TreeTable pinned section', () => {
  beforeEach(mockLayout);
  afterEach(restoreLayout);

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

  it("draws a pinned row's children beneath it as part of the same pinned item", () => {
    renderTable({
      pinnedItems: [{ ...row('x'), children: [row('x-child')] }, row('y')],
    });

    const items = within(screen.getByRole('region', { name: 'Pinned rows' })).getAllByRole(
      'listitem'
    );
    expect(items).toHaveLength(2);
    const first = within(items[0]!);
    expect(first.getByText('Row x')).toBeInTheDocument();
    expect(first.getByText('Row x-child')).toBeInTheDocument();
    expect(first.getByText('Row x').compareDocumentPosition(first.getByText('Row x-child'))).toBe(
      Node.DOCUMENT_POSITION_FOLLOWING
    );
    expect(within(items[1]!).queryByText('Row x-child')).not.toBeInTheDocument();
  });

  it("opens a pinned row's nested row with a caret that follows the shared expanded state", async () => {
    const onExpandChange = vi.fn();
    const pinnedItems = [
      { ...row('x'), children: [{ ...row('x-disclosure'), children: [row('x-chart')] }] },
    ];
    const { rerender } = render(
      <TreeTable<Row>
        data={[row('a')]}
        columns={columns}
        virtualized
        rowHeight={ROW_HEIGHT}
        pinnedItems={pinnedItems}
        controlledExpandedIds={new Set()}
        onExpandChange={onExpandChange}
      />
    );

    const region = within(screen.getByRole('region', { name: 'Pinned rows' }));
    expect(region.getByText('Row x-disclosure')).toBeInTheDocument();
    expect(region.queryByText('Row x-chart')).not.toBeInTheDocument();

    await userEvent.click(region.getByText('Row x-disclosure'));
    expect(onExpandChange).toHaveBeenCalledWith('x-disclosure', true);

    rerender(
      <TreeTable<Row>
        data={[row('a')]}
        columns={columns}
        virtualized
        rowHeight={ROW_HEIGHT}
        pinnedItems={pinnedItems}
        controlledExpandedIds={new Set(['x-disclosure'])}
        onExpandChange={onExpandChange}
      />
    );
    expect(region.getByText('Row x-chart')).toBeInTheDocument();

    await userEvent.click(region.getByText('Row x-disclosure'));
    expect(onExpandChange).toHaveBeenLastCalledWith('x-disclosure', false);
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

  it('clears the pins from a button beside the header without toggling the section', async () => {
    const onClearPinned = vi.fn();
    const onTogglePinnedCollapsed = vi.fn();
    renderTable({ pinnedItems: [row('x')], onClearPinned, onTogglePinnedCollapsed });

    await userEvent.click(screen.getByRole('button', { name: 'Clear pins' }));

    expect(onClearPinned).toHaveBeenCalledTimes(1);
    expect(onTogglePinnedCollapsed).not.toHaveBeenCalled();
  });

  it('keeps the clear button when the section is collapsed', () => {
    renderTable({ pinnedItems: [row('x')], isPinnedCollapsed: true, onClearPinned: vi.fn() });

    expect(screen.getByRole('button', { name: 'Clear pins' })).toBeInTheDocument();
  });

  it('offers to clear when every pin is hidden by a filter', () => {
    renderTable({ pinnedItems: [], hiddenPinnedCount: 2, onClearPinned: vi.fn() });

    expect(screen.getByRole('button', { name: 'Clear pins' })).toBeInTheDocument();
  });

  it('has no clear button unless a handler is given', () => {
    renderTable({ pinnedItems: [row('x')] });

    expect(screen.queryByRole('button', { name: 'Clear pins' })).not.toBeInTheDocument();
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

  it('shows a small set of pinned rows in full, with no height cap', () => {
    const pinnedItems = Array.from({ length: 4 }, (_, i) => row(`p${i}`));
    renderTable({ pinnedItems });

    const list = screen.getByRole('list', { name: 'Pinned rows list' });
    expect(list.style.maxHeight).toBe('');
    expect(within(list).getAllByRole('listitem')).toHaveLength(4);
  });

  it('renders only a window of the pinned rows when there are many', () => {
    const pinnedItems = Array.from({ length: 200 }, (_, i) => row(`p${i}`));
    renderTable({ pinnedItems });

    const list = screen.getByRole('list', { name: 'Pinned rows list' });
    const rendered = within(list).getAllByRole('listitem');
    expect(rendered.length).toBeGreaterThan(0);
    expect(rendered.length).toBeLessThan(50);
    // The list is still as tall as every row, so the scrollbar is the right size.
    expect(list.style.height).toBe(`${200 * ROW_HEIGHT}px`);
    // Rows know where they sit in the whole list, not just in the rendered window.
    expect(rendered[0]).toHaveAttribute('aria-posinset', '1');
    expect(rendered[0]).toHaveAttribute('aria-setsize', '200');
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
  beforeEach(mockLayout);
  afterEach(restoreLayout);

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
  beforeEach(mockLayout);
  afterEach(restoreLayout);

  it('is a named hover group so controls in it can ignore hovering the other columns', () => {
    renderTable({ pinnedItems: [row('x')] });

    const cell = within(screen.getByRole('region', { name: 'Pinned rows' }))
      .getByText('Row x')
      .closest('.group\\/info');

    expect(cell).not.toBeNull();
  });
});

describe('TreeTable variable row heights', () => {
  it('uses a per-row height override in virtualized trees', () => {
    const parent = { ...row('parent'), children: [row('child')] };
    const getRowHeight = vi.fn((item: { id: string }) => (item.id === 'parent' ? 24 : ROW_HEIGHT));
    render(
      <TreeTable<Row>
        data={[parent]}
        columns={columns}
        controlledExpandedIds={new Set(['parent'])}
        getRowHeight={getRowHeight}
        rowHeight={ROW_HEIGHT}
        virtualized
      />
    );

    expect(screen.getByRole('tree')).toHaveStyle({ height: `${24 + ROW_HEIGHT}px` });
    expect(getRowHeight.mock.calls.map(([item]) => item.id)).toEqual(
      expect.arrayContaining(['parent', 'child'])
    );
  });
});

describe('TreeTable rows that change height', () => {
  const CHART_HEIGHT = 128;

  beforeEach(() => {
    Object.defineProperty(Element.prototype, 'clientHeight', {
      configurable: true,
      get(this: Element) {
        return this.hasAttribute('data-tree-table-scroller') ? 600 : 0;
      },
    });
    // Like a browser, a row is as tall as its minimum height or its content, whichever is more.
    Object.defineProperty(HTMLElement.prototype, 'offsetHeight', {
      configurable: true,
      get(this: HTMLElement) {
        if (!this.hasAttribute('data-index')) {
          return 0;
        }
        const chart = this.querySelector<HTMLElement>('[data-chart]');
        return Math.max(parseFloat(this.style.minHeight) || 0, chart ? CHART_HEIGHT : 0);
      },
    });
  });
  afterEach(restoreLayout);

  it('moves the rows below right away when a row grows to fit new content', () => {
    const parent = { ...row('parent'), children: [row('child')] };
    const treeTable = (isChartShown: boolean) => (
      <TreeTable<Row>
        data={[parent]}
        columns={[
          {
            ...columns[0]!,
            render: ({ item }) =>
              item.id === 'parent' && isChartShown ? (
                <div data-chart style={{ height: CHART_HEIGHT }} />
              ) : (
                <span>{item.name}</span>
              ),
          },
        ]}
        controlledExpandedIds={new Set(['parent'])}
        // The row's minimum height is less than its chart, as the Operators chart's is.
        getRowHeight={item => (item.id === 'parent' && isChartShown ? 45 : 28)}
        rowHeight={ROW_HEIGHT}
        virtualized
      />
    );
    const { rerender, container } = render(treeTable(false));
    expect(screen.getByRole('tree')).toHaveStyle({ height: `${28 + 28}px` });

    rerender(treeTable(true));

    // jsdom never reports a resize, so the table has to measure the row itself.
    expect(screen.getByRole('tree')).toHaveStyle({ height: `${CHART_HEIGHT + 28}px` });
    expect(container.querySelector('[data-index="1"]')).toHaveStyle({
      transform: `translateY(${CHART_HEIGHT}px)`,
    });
  });
});

describe('TreeTable scroller', () => {
  it('marks its scroll container so callers can find it', () => {
    const { container } = renderTable();

    expect(container.querySelectorAll(TREE_TABLE_SCROLLER_SELECTOR)).toHaveLength(1);
  });
});
