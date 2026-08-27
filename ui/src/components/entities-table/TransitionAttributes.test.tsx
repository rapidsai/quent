// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { TransitionAttributes } from './TransitionAttributes';

describe('TransitionAttributes', () => {
  it('groups recorded and derived attributes into separate boxes', () => {
    const operatorLabel = vi.fn(() => 'Scan operator');

    render(
      <TransitionAttributes
        attributes={[
          { key: 'operator_id', value: 'operator-1' },
          { key: 'attempt', value: 2 },
        ]}
        derivedAttributes={[{ key: 'output_bytes', value: 2048n }]}
        operatorLabel={operatorLabel}
      />
    );

    expect(screen.getByRole('heading', { name: 'Attributes' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Derived attributes' })).toBeInTheDocument();
    expect(screen.getByText('operator')).toBeInTheDocument();
    expect(screen.getByText('Scan operator')).toBeInTheDocument();
    expect(screen.getByText('attempt')).toBeInTheDocument();
    expect(screen.getByText('2')).toBeInTheDocument();
    expect(screen.getByText('output_bytes')).toBeInTheDocument();
    expect(screen.getByText('2.00 KiB')).toBeInTheDocument();
    expect(operatorLabel).toHaveBeenCalledWith('operator-1');
  });
});
