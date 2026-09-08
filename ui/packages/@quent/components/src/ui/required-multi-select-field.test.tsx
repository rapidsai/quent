// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { RequiredMultiSelectField } from './required-multi-select-field';

const OPTIONS = [
  { value: 'memory', label: 'Memory' },
  { value: 'filesystem', label: 'Filesystem' },
];

describe('RequiredMultiSelectField', () => {
  it('prevents deselecting the final option', () => {
    render(
      <RequiredMultiSelectField
        label="Data location"
        options={OPTIONS}
        selected={new Set(['memory'])}
        onToggle={vi.fn()}
      />
    );

    expect(screen.getByRole('checkbox', { name: 'Memory' })).toBeDisabled();
    expect(screen.getByText('At least one option is required.', { exact: false })).toBeVisible();
  });

  it('allows adding another option', () => {
    const onToggle = vi.fn();
    render(
      <RequiredMultiSelectField
        label="Data location"
        options={OPTIONS}
        selected={new Set(['memory'])}
        onToggle={onToggle}
      />
    );

    fireEvent.click(screen.getByRole('checkbox', { name: 'Filesystem' }));

    expect(onToggle).toHaveBeenCalledOnce();
    expect(onToggle).toHaveBeenCalledWith('filesystem');
  });
});
