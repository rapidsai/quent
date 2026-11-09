// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { PinnedRowsSplit } from './PinnedRowsSplit';
import { measurePinnedPanelHeights } from './pinnedPanelHeights';

vi.mock('./pinnedPanelHeights', () => ({ measurePinnedPanelHeights: vi.fn() }));

const measure = vi.mocked(measurePinnedPanelHeights);

function renderSplit(props: { pinned?: boolean; isCollapsed?: boolean } = {}) {
  const { pinned = true, isCollapsed = false } = props;
  return render(
    <PinnedRowsSplit
      pinned={pinned ? <div>pinned rows</div> : undefined}
      pinnedHeader={pinned ? <div>pinned header</div> : undefined}
      pinnedCount={pinned ? 1 : 0}
      isCollapsed={isCollapsed}
    >
      <div>main rows</div>
    </PinnedRowsSplit>
  );
}

describe('PinnedRowsSplit', () => {
  beforeEach(() => {
    measure.mockReset();
  });

  it('measures the panel content and, separately, its header', () => {
    renderSplit();

    expect(measure).toHaveBeenCalled();
    const [content, header] = measure.mock.calls[0];
    expect(content).toContainElement(screen.getByText('pinned rows'));
    expect(header).toContainElement(screen.getByText('pinned header'));
    expect(header).not.toContainElement(screen.getByText('pinned rows'));
    expect(content).toContainElement(header);
  });

  it('shows the pinned rows above the main rows with a handle between them', () => {
    renderSplit();

    const pinned = screen.getByText('pinned rows');
    const main = screen.getByText('main rows');
    expect(pinned.compareDocumentPosition(main) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(screen.getByRole('separator')).toBeInTheDocument();
  });

  it('shows only the main rows, without a handle, when nothing is pinned', () => {
    renderSplit({ pinned: false });

    expect(screen.getByText('main rows')).toBeInTheDocument();
    expect(screen.queryByRole('separator')).not.toBeInTheDocument();
    expect(measure).not.toHaveBeenCalled();
  });

  it('cannot be resized by hand while collapsed', () => {
    renderSplit({ isCollapsed: true });

    expect(screen.getByRole('separator')).toHaveAttribute('aria-disabled', 'true');
  });
});
