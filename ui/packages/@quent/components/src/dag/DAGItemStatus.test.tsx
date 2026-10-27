// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { DAGItemStatus } from './DAGItemStatus';

describe('DAGItemStatus', () => {
  it('shows pin, filter, and combined status', () => {
    const { rerender } = render(<DAGItemStatus label="Scan" pinned filtered={false} />);
    expect(screen.getByRole('status', { name: 'Scan: pinned' })).toBeVisible();

    rerender(<DAGItemStatus label="Scan" pinned={false} filtered />);
    expect(screen.getByRole('status', { name: 'Scan: filtered' })).toBeVisible();

    rerender(<DAGItemStatus label="Scan" pinned filtered />);
    expect(screen.getByRole('status', { name: 'Scan: pinned and filtered' })).toBeVisible();
  });

  it('renders nothing without an active state', () => {
    const { container } = render(<DAGItemStatus label="Scan" pinned={false} />);
    expect(container).toBeEmptyDOMElement();
  });
});
