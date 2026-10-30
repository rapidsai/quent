// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { DAGItemStatus } from './DAGItemStatus';

describe('DAGItemStatus', () => {
  it('shows filter and pin statuses', () => {
    const { rerender } = render(<DAGItemStatus label="Scan" filtered />);
    const filtered = screen.getByRole('status', { name: 'Scan: filtered' });
    expect(filtered).toBeVisible();
    expect(filtered).not.toHaveClass('border', 'bg-card/90', 'shadow-sm');
    expect(filtered.querySelector('svg')).toHaveClass('lucide-filter', 'opacity-40');

    rerender(<DAGItemStatus label="Pipe" pinned />);
    const pinned = screen.getByRole('status', { name: 'Pipe: pinned' });
    expect(pinned.querySelector('svg')).toHaveClass('lucide-pin');

    rerender(<DAGItemStatus label="Scan" pinned filtered />);
    const combined = screen.getByRole('status', {
      name: 'Scan: pinned and filtered',
    });
    expect(combined).toBeVisible();
    expect(combined.querySelectorAll('svg')).toHaveLength(2);
  });

  it('renders nothing without an active state', () => {
    const { container } = render(<DAGItemStatus label="Scan" />);
    expect(container).toBeEmptyDOMElement();
  });
});
