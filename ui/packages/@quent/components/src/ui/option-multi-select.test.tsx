// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { OptionMultiSelect } from './option-multi-select';

describe('OptionMultiSelect', () => {
  it('shows a short label while exposing the full path as a title', async () => {
    const user = userEvent.setup();
    render(
      <OptionMultiSelect
        ariaLabel="Columns"
        triggerText="Choose columns"
        options={[
          {
            value: 'input-rows',
            label: 'rows',
            title: 'Execution profile › Data volume › Input › rows',
          },
        ]}
        selectedOptionIds={new Set()}
        onToggleOption={vi.fn()}
        onSelectAllOptions={vi.fn()}
        onSelectNoOptions={vi.fn()}
      />
    );

    await user.click(screen.getByRole('combobox', { name: 'Columns' }));

    expect(screen.getByText('rows')).toHaveAttribute(
      'title',
      'Execution profile › Data volume › Input › rows'
    );
  });
});
