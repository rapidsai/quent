// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { MAX_PINNED_ROWS } from '@/features/pinned-rows/pinnedRows';
import { PinnableRowLabel } from './PinnableRowLabel';

function renderLabel(props: Partial<React.ComponentProps<typeof PinnableRowLabel>> = {}) {
  const onTogglePin = vi.fn();
  const onRowClick = vi.fn();
  const { container } = render(
    // The parent click stands in for a tree row, which expands and selects on click.
    <div onClick={onRowClick}>
      <PinnableRowLabel
        label={<span>GPU 0</span>}
        name="GPU 0"
        isPinnable
        isPinned={false}
        isAtPinLimit={false}
        onTogglePin={onTogglePin}
        {...props}
      />
    </div>
  );
  return { onTogglePin, onRowClick, container };
}

describe('PinnableRowLabel', () => {
  it('renders the row label', () => {
    renderLabel();

    expect(screen.getByText('GPU 0')).toBeInTheDocument();
  });

  it('offers a pin button that is not pressed for an unpinned row', () => {
    renderLabel();

    expect(screen.getByRole('button', { name: 'Pin GPU 0' })).toHaveAttribute(
      'aria-pressed',
      'false'
    );
  });

  it('offers an unpin button that is pressed for a pinned row', () => {
    renderLabel({ isPinned: true });

    expect(screen.getByRole('button', { name: 'Unpin GPU 0' })).toHaveAttribute(
      'aria-pressed',
      'true'
    );
  });

  it('has no button when the row cannot be pinned', () => {
    renderLabel({ isPinnable: false });

    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });

  it('toggles the pin without triggering the row click', async () => {
    const { onTogglePin, onRowClick } = renderLabel();

    await userEvent.click(screen.getByRole('button', { name: 'Pin GPU 0' }));

    expect(onTogglePin).toHaveBeenCalledTimes(1);
    expect(onRowClick).not.toHaveBeenCalled();
  });

  it('disables pinning more rows once the limit is reached', () => {
    renderLabel({ isAtPinLimit: true });

    const button = screen.getByRole('button', { name: 'Pin GPU 0' });
    expect(button).toBeDisabled();
    expect(button).toHaveAttribute('title', `You can pin up to ${MAX_PINNED_ROWS} rows`);
  });

  it('still lets a pinned row be unpinned at the limit', () => {
    renderLabel({ isAtPinLimit: true, isPinned: true });

    expect(screen.getByRole('button', { name: 'Unpin GPU 0' })).toBeEnabled();
  });

  it('appears when the info area is hovered, not when anywhere else on the row is', () => {
    renderLabel();

    const classes = screen.getByRole('button', { name: 'Pin GPU 0' }).className;

    expect(classes).toContain('group-hover/info:opacity-100');
    expect(classes).not.toMatch(/(^|\s)group-hover:opacity-100/);
  });
});
