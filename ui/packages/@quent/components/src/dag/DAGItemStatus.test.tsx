// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { DAGItemStatus } from './DAGItemStatus';

describe('DAGItemStatus', () => {
  it('shows pin or filter status without a redundant pin for filtered items', () => {
    const { rerender } = render(<DAGItemStatus label="Scan" pinned filtered={false} />);
    const pinned = screen.getByRole('status', { name: 'Scan: pinned' });
    expect(pinned).toBeVisible();
    expect(pinned).not.toHaveClass('border', 'bg-card/90', 'shadow-sm');

    rerender(<DAGItemStatus label="Scan" pinned={false} filtered />);
    expect(screen.getByRole('status', { name: 'Scan: filtered' })).toBeVisible();

    rerender(<DAGItemStatus label="Scan" pinned filtered />);
    const combined = screen.getByRole('status', { name: 'Scan: pinned and filtered' });
    expect(combined).toBeVisible();
    const icons = combined.querySelectorAll('svg');
    expect(icons[0]).toHaveClass('lucide-filter');
    expect(icons).toHaveLength(1);
  });

  it('renders nothing without an active state', () => {
    const { container } = render(<DAGItemStatus label="Scan" pinned={false} />);
    expect(container).toBeEmptyDOMElement();
  });
});
