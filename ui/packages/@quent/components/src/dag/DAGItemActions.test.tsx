// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { DAGItemActions } from './DAGItemActions';

describe('DAGItemActions', () => {
  it('offers preview, pin, and operator filtering without activating the item', async () => {
    const user = userEvent.setup();
    const activateItem = vi.fn();
    const togglePin = vi.fn();
    const toggleFilter = vi.fn();
    render(
      <div onClick={activateItem}>
        <DAGItemActions
          label="Scan"
          pinned={false}
          filtered={false}
          visible
          previewContent={<div>Scan preview</div>}
          onTogglePin={togglePin}
          onToggleFilter={toggleFilter}
        />
      </div>
    );

    const preview = screen.getByRole('button', { name: 'Preview details for Scan' });
    const pin = screen.getByRole('button', { name: 'Pin details for Scan' });
    const filter = screen.getByRole('button', { name: 'Filter operator Scan' });

    expect(pin).toHaveAttribute('aria-pressed', 'false');
    expect(filter).toHaveAttribute('aria-pressed', 'false');

    await user.click(preview);
    expect(activateItem).not.toHaveBeenCalled();
    expect(screen.getByText('Scan preview')).toBeVisible();

    await user.click(pin);
    expect(togglePin).toHaveBeenCalledOnce();

    await user.click(filter);
    expect(toggleFilter).toHaveBeenCalledOnce();
  });

  it('omits filtering for edges and exposes active labels', async () => {
    const user = userEvent.setup();
    render(
      <DAGItemActions
        label="out to in"
        pinned
        filtered
        visible
        previewContent={<div>Pipe preview</div>}
        onTogglePin={() => undefined}
      />
    );

    expect(screen.getByRole('button', { name: 'Pin details for out to in' })).toHaveAttribute(
      'aria-pressed',
      'true'
    );
    expect(screen.queryByRole('button', { name: /filter/i })).not.toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Preview details for out to in' }));
    expect(screen.getByText('Pipe preview')).toBeVisible();
  });

  it('stays hidden until hover or focus and stretches each action equally', () => {
    render(
      <DAGItemActions
        label="Scan"
        pinned={false}
        filtered={false}
        previewContent={<div>Scan preview</div>}
        onTogglePin={() => undefined}
        onToggleFilter={() => undefined}
      />
    );

    const toolbar = screen.getByRole('toolbar', { name: 'Actions for Scan' });
    expect(toolbar).toHaveClass('w-full', 'opacity-0', 'group-hover:opacity-100');
    for (const button of within(toolbar).getAllByRole('button')) {
      expect(button).toHaveClass('flex-1', 'cursor-pointer');
    }
  });

  it('closes the preview when clicking elsewhere', async () => {
    const user = userEvent.setup();
    render(
      <>
        <DAGItemActions
          label="Scan"
          pinned={false}
          visible
          previewContent={<div>Scan preview</div>}
          onTogglePin={() => undefined}
        />
        <button type="button">DAG background</button>
      </>
    );

    await user.click(screen.getByRole('button', { name: 'Preview details for Scan' }));
    expect(screen.getByText('Scan preview')).toBeVisible();

    await user.click(screen.getByRole('button', { name: 'DAG background' }));
    expect(screen.queryByText('Scan preview')).not.toBeInTheDocument();
  });
});
