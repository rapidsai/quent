// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { DAGItemActions } from './DAGItemActions';

describe('DAGItemActions', () => {
  it('offers preview and operator filtering without activating the item', async () => {
    const user = userEvent.setup();
    const activateItem = vi.fn();
    const toggleFilter = vi.fn();
    render(
      <div onClick={activateItem}>
        <DAGItemActions
          label="Scan"
          filtered
          visible
          previewContent={<div>Scan preview</div>}
          onToggleFilter={toggleFilter}
        />
      </div>
    );

    const preview = screen.getByRole('button', { name: 'Preview details for Scan' });
    const filter = screen.getByRole('button', { name: 'Filter operator Scan' });

    expect(filter).toHaveAttribute('aria-pressed', 'true');
    expect(filter.getAttribute('style')).toContain('42%');
    expect(filter.getAttribute('style')).toContain('54%');
    expect(screen.queryByRole('button', { name: /pin details/i })).not.toBeInTheDocument();

    await user.click(preview);
    expect(activateItem).not.toHaveBeenCalled();
    expect(screen.getByText('Scan preview')).toBeVisible();

    await user.click(filter);
    expect(toggleFilter).toHaveBeenCalledOnce();
  });

  it('offers only preview for edges', async () => {
    const user = userEvent.setup();
    render(
      <DAGItemActions
        label="out to in"
        visible
        backgroundColor="#123456"
        previewContent={<div>Pipe preview</div>}
      />
    );

    const preview = screen.getByRole('button', { name: 'Preview details for out to in' });
    expect(preview.getAttribute('style')).toContain('14%');
    expect(preview.getAttribute('style')).toContain('26%');
    expect(screen.queryByRole('button', { name: /pin/i })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /filter/i })).not.toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Preview details for out to in' }));
    expect(screen.getByText('Pipe preview')).toBeVisible();
  });

  it('stays hidden until hover or focus and stretches each action equally', () => {
    render(
      <DAGItemActions
        label="Scan"
        filtered={false}
        divided={false}
        previewContent={<div>Scan preview</div>}
        onToggleFilter={() => undefined}
      />
    );

    const toolbar = screen.getByRole('toolbar', { name: 'Actions for Scan' });
    expect(toolbar).toHaveClass('w-full', 'opacity-0', 'group-hover:opacity-100');
    expect(toolbar).not.toHaveClass('divide-x');
    for (const button of within(toolbar).getAllByRole('button')) {
      expect(button).toHaveClass('flex-1', 'cursor-pointer');
    }
  });

  it('closes the preview when clicking elsewhere', async () => {
    const user = userEvent.setup();
    render(
      <>
        <DAGItemActions label="Scan" visible previewContent={<div>Scan preview</div>} />
        <button type="button">DAG background</button>
      </>
    );

    await user.click(screen.getByRole('button', { name: 'Preview details for Scan' }));
    expect(screen.getByText('Scan preview')).toBeVisible();

    await user.click(screen.getByRole('button', { name: 'DAG background' }));
    expect(screen.queryByText('Scan preview')).not.toBeInTheDocument();
  });
});
