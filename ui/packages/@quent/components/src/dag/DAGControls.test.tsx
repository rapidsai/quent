// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Provider, createStore } from 'jotai';
import { describe, expect, it } from 'vitest';
import { useSelectedColorField } from '@quent/hooks';
import { flattenStatistics, statisticFieldId } from '@quent/utils';
import { DAGControls } from './DAGControls';

function Selection() {
  const [field] = useSelectedColorField();
  return <output data-testid="selected-field">{field}</output>;
}

describe('DAGControls statistic selection', () => {
  it('selects nested and literal separator-containing names independently', async () => {
    const user = userEvent.setup();
    const fields = flattenStatistics([
      { key: 'Volume', value: { kind: 'struct', fields: [{ key: 'bytes', value: 1 }] } },
      { key: 'Volume › bytes', value: 2 },
    ]);
    render(
      <Provider store={createStore()}>
        <DAGControls operatorStatFields={fields} portStatFields={fields} isDark={false} />
        <Selection />
      </Provider>
    );

    await user.click(screen.getByRole('combobox', { name: 'Node color' }));
    await user.click(screen.getByRole('option', { name: 'Volume › bytes', exact: true }));
    expect(screen.getByTestId('selected-field')).toHaveTextContent(
      statisticFieldId([
        ['Volume', 0],
        ['bytes', 0],
      ])
    );
    expect(screen.getByRole('combobox', { name: 'Node color' })).toHaveTextContent(
      'Volume › bytes'
    );

    await user.click(screen.getByRole('combobox', { name: 'Node color' }));
    await user.click(screen.getByRole('option', { name: '"Volume › bytes"', exact: true }));
    expect(screen.getByTestId('selected-field')).toHaveTextContent('Volume › bytes');
    expect(screen.getByRole('combobox', { name: 'Node color' })).toHaveTextContent(
      '"Volume › bytes"'
    );
  });
});
