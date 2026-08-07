// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { ControlField, ControlGrid, ControlSection } from './control-grid';

describe('ControlGrid', () => {
  it('renders its children', () => {
    render(
      <ControlGrid columns={2}>
        <span>Left</span>
        <span>Right</span>
      </ControlGrid>
    );

    expect(screen.getByText('Left')).toBeVisible();
    expect(screen.getByText('Right')).toBeVisible();
  });

  it('composes sections and labeled fields', () => {
    render(
      <ControlSection
        title="Display"
        description="Tune appearance"
        action={<button type="button">Reset</button>}
      >
        <ControlGrid>
          <ControlField label="Name">
            <input aria-label="Name value" />
          </ControlField>
        </ControlGrid>
      </ControlSection>
    );

    expect(screen.getByRole('heading', { name: 'Display' })).toBeVisible();
    expect(screen.getByText('Tune appearance')).toBeVisible();
    expect(screen.getByRole('button', { name: 'Reset' })).toBeVisible();
    expect(screen.getByText('Name')).toBeVisible();
    expect(screen.getByRole('textbox', { name: 'Name value' })).toBeVisible();
  });

  it('renders a trailing adornment only when provided', () => {
    render(
      <>
        <ControlField label="Without adornment">
          <input aria-label="Without adornment value" />
        </ControlField>
        <ControlField
          label="With adornment"
          trailingAdornment={<button type="button">Pick color</button>}
        >
          <input aria-label="With adornment value" />
        </ControlField>
      </>
    );

    expect(screen.queryByRole('button', { name: 'Pick color' })).toBeVisible();
    expect(screen.getByRole('textbox', { name: 'Without adornment value' })).toBeVisible();
    expect(screen.getByRole('textbox', { name: 'With adornment value' })).toBeVisible();
    expect(screen.getAllByRole('button')).toHaveLength(1);
  });
});
