// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { EmptyState } from './empty-state';

describe('EmptyState', () => {
  it('renders the title, description and action', () => {
    render(
      <EmptyState
        title="No data"
        description="Nothing matches the filters."
        action={<button type="button">Reset</button>}
      />
    );

    expect(screen.getByText('No data')).toBeInTheDocument();
    expect(screen.getByText('Nothing matches the filters.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Reset' })).toBeInTheDocument();
  });

  it('omits the optional parts when they are not given', () => {
    render(<EmptyState title="No data" />);

    expect(screen.getByText('No data')).toBeInTheDocument();
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });

  it('renders a custom icon in the default size only', () => {
    const Icon = ({ className }: { className?: string }) => (
      <svg data-testid="icon" className={className} />
    );
    const { rerender } = render(<EmptyState title="No data" icon={Icon} />);
    expect(screen.getByTestId('icon')).toBeInTheDocument();

    rerender(<EmptyState title="No data" icon={Icon} size="compact" />);
    expect(screen.queryByTestId('icon')).not.toBeInTheDocument();
  });
});
