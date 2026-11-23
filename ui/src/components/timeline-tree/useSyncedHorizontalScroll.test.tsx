// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useRef } from 'react';
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { useSyncedHorizontalScroll } from './useSyncedHorizontalScroll';

function Tables({ enabled = true }: { enabled?: boolean }) {
  const containerRef = useRef<HTMLDivElement>(null);
  useSyncedHorizontalScroll(containerRef, enabled);
  return (
    <div ref={containerRef}>
      <div data-testid="first" data-tree-table-scroller />
      <div data-testid="second" data-tree-table-scroller />
      <div data-testid="unrelated" className="overflow-auto" />
    </div>
  );
}

function scrollTo(testId: string, left: number) {
  const element = screen.getByTestId(testId);
  element.scrollLeft = left;
  fireEvent.scroll(element);
}

describe('useSyncedHorizontalScroll', () => {
  it('mirrors the horizontal offset onto the other table scrollers', () => {
    render(<Tables />);

    scrollTo('first', 40);

    expect(screen.getByTestId('second').scrollLeft).toBe(40);
  });

  it('ignores scrollable elements that are not table scrollers', () => {
    render(<Tables />);

    scrollTo('first', 40);
    expect(screen.getByTestId('unrelated').scrollLeft).toBe(0);

    scrollTo('unrelated', 25);
    expect(screen.getByTestId('first').scrollLeft).toBe(40);
  });

  it('does nothing until enabled', () => {
    render(<Tables enabled={false} />);

    scrollTo('first', 40);

    expect(screen.getByTestId('second').scrollLeft).toBe(0);
  });

  it('starts syncing once it becomes enabled', () => {
    const { rerender } = render(<Tables enabled={false} />);

    rerender(<Tables enabled />);
    scrollTo('first', 40);

    expect(screen.getByTestId('second').scrollLeft).toBe(40);
  });
});
