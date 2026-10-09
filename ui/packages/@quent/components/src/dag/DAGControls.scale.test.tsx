// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Provider, createStore } from 'jotai';
import { describe, expect, it } from 'vitest';
import { useSelectedScaleType } from '@quent/hooks';
import { DAGControls } from './DAGControls';

function ScaleSelection() {
  const [scale] = useSelectedScaleType();
  return <output data-testid="selected-scale">{scale}</output>;
}

function renderControls() {
  return render(
    <Provider store={createStore()}>
      <DAGControls operatorStatFields={[]} portStatFields={[]} isDark={false} />
      <ScaleSelection />
    </Provider>
  );
}

describe('DAGControls edge scale', () => {
  it('offers an edge scale dropdown that defaults to log', () => {
    renderControls();
    expect(screen.getByRole('combobox', { name: 'Scale' })).toHaveTextContent('Log');
    expect(screen.getByTestId('selected-scale')).toHaveTextContent('log');
  });

  it('places the edge scale dropdown immediately before layout direction', () => {
    renderControls();
    const comboboxes = screen.getAllByRole('combobox');
    const scale = screen.getByRole('combobox', { name: 'Scale' });
    const layout = screen.getByRole('combobox', { name: 'Layout direction' });
    expect(comboboxes.indexOf(layout) - comboboxes.indexOf(scale)).toBe(1);
  });

  it('switches the scale to linear', async () => {
    const user = userEvent.setup();
    renderControls();
    await user.click(screen.getByRole('combobox', { name: 'Scale' }));
    await user.click(screen.getByRole('option', { name: 'Linear' }));
    expect(screen.getByTestId('selected-scale')).toHaveTextContent('linear');
    expect(screen.getByRole('combobox', { name: 'Scale' })).toHaveTextContent('Linear');
  });
});
