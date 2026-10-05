// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { Badge } from './badge';
import { TruncatedBadgeList } from './truncated-badge-list';

describe('TruncatedBadgeList', () => {
  it('lists hidden item labels in the overflow badge title', () => {
    render(
      <TruncatedBadgeList
        items={['Alpha', 'Beta', 'Gamma']}
        maxVisible={1}
        getItemKey={item => item}
        getItemLabel={item => item}
        renderBadge={item => <Badge>{item}</Badge>}
      />
    );

    expect(screen.getByText('Alpha')).toBeInTheDocument();
    expect(screen.queryByText('Beta')).not.toBeInTheDocument();
    expect(screen.getByText('+2 more')).toHaveAttribute('title', 'Beta, Gamma');
  });
});

const ITEMS = Array.from({ length: 10 }, (_, index) => `Item ${index + 1}`);
const ITEM_WIDTH = 100;
let itemWidth = ITEM_WIDTH;

function fitList(items: readonly string[]) {
  return (
    <TruncatedBadgeList
      items={items}
      maxVisible={50}
      fitToWidth
      getItemKey={item => item}
      getItemLabel={item => item}
      renderBadge={item => <span>{item}</span>}
      renderOverflowLabel={hiddenCount => `and ${hiddenCount} more`}
      trailing={<button type="button">Clear</button>}
    />
  );
}

function renderList(containerWidth: number) {
  // jsdom has no layout, so give every measured element a fixed width.
  vi.spyOn(HTMLElement.prototype, 'offsetWidth', 'get').mockImplementation(() => itemWidth);
  vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockReturnValue(containerWidth);
  const view = render(fitList(ITEMS));
  return { rerenderWithNewItems: () => view.rerender(fitList([...ITEMS])) };
}

describe('TruncatedBadgeList fitToWidth', () => {
  afterEach(() => {
    vi.restoreAllMocks();
    itemWidth = ITEM_WIDTH;
  });

  it('shows only the badges that fit next to the overflow badge and trailing content', () => {
    // 3 badges + overflow + trailing = 5 pieces of 100px each.
    renderList(500);

    expect(screen.getByText('Item 3')).toBeInTheDocument();
    expect(screen.queryByText('Item 4')).not.toBeInTheDocument();
    expect(screen.getByText('and 7 more')).toHaveAttribute('title', ITEMS.slice(3).join(', '));
    expect(screen.getByRole('button', { name: 'Clear' })).toBeInTheDocument();
  });

  it('shows every badge and no overflow when they all fit', () => {
    renderList(5000);

    expect(screen.getByText('Item 10')).toBeInTheDocument();
    expect(screen.queryByText(/more$/)).not.toBeInTheDocument();
  });

  it('always keeps one badge visible, even in a tiny container', () => {
    renderList(10);

    expect(screen.getByText('Item 1')).toBeInTheDocument();
    expect(screen.queryByText('Item 2')).not.toBeInTheDocument();
  });

  it('measures again when the items change, even if the keys stay the same', () => {
    const { rerenderWithNewItems } = renderList(500);
    expect(screen.queryByText('Item 4')).not.toBeInTheDocument();

    itemWidth = 50;
    rerenderWithNewItems();

    expect(screen.getByText('Item 8')).toBeInTheDocument();
    expect(screen.queryByText('Item 9')).not.toBeInTheDocument();
  });
});
