// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { DAGItemStatus } from './DAGItemStatus';

describe('DAGItemStatus', () => {
  it('shows filter and pin statuses', () => {
    const { rerender } = render(<DAGItemStatus label="Scan" status="filtered" />);
    const filtered = screen.getByRole('status', { name: 'Scan: filtered' });
    expect(filtered).toBeVisible();
    expect(filtered).toHaveClass('text-foreground');
    expect(filtered).not.toHaveClass('border', 'bg-card/90', 'shadow-sm');
    expect(filtered.querySelector('svg')).toHaveClass('lucide-filter', 'opacity-40');

    rerender(<DAGItemStatus label="Pipe" status="pinned" />);
    const pinned = screen.getByRole('status', { name: 'Pipe: pinned' });
    expect(pinned.querySelector('svg')).toHaveClass('lucide-pin', 'opacity-40');
  });
});
