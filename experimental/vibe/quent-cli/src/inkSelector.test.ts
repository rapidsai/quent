// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { act, createElement } from 'react';
import { renderToString } from 'ink';
import { SelectionPrompt } from './inkSelectionPrompt';
import { filterSelectionChoices, selectionWindow } from './inkSelector.utils';

const choices = [
  { value: 'engine-alpha', label: 'Alpha Engine' },
  { value: 'engine-beta', label: 'Beta Engine' },
  { value: 'warehouse', label: 'Production Warehouse' },
];

describe('Ink selector', () => {
  it('renders branded instructions and selectable choices', async () => {
    let output = '';
    await act(() => {
      output = renderToString(
        createElement(SelectionPrompt, {
          prompt: 'Select an engine',
          choices,
        })
      );
    });

    expect(output).toContain('QUENT');
    expect(output).toContain('Select an engine');
    expect(output).toContain('› Alpha Engine');
    expect(output).toContain('type to filter');
  });

  it('filters case-insensitively across labels and IDs', () => {
    expect(filterSelectionChoices(choices, 'WARE')).toEqual([choices[2]]);
    expect(filterSelectionChoices(choices, 'engine-beta')).toEqual([choices[1]]);
    expect(filterSelectionChoices(choices, '')).toBe(choices);
  });

  it('keeps the selected choice in a bounded scrolling window', () => {
    const manyChoices = Array.from({ length: 20 }, (_, index) => ({
      value: String(index),
      label: `Choice ${index}`,
    }));

    expect(selectionWindow(manyChoices, 10, 5)).toEqual({
      choices: manyChoices.slice(8, 13),
      offset: 8,
    });
    expect(selectionWindow(manyChoices, 19, 5)).toEqual({
      choices: manyChoices.slice(15, 20),
      offset: 15,
    });
  });
});
