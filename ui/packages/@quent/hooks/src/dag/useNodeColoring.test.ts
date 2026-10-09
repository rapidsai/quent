// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createElement, type ReactNode } from 'react';
import { renderHook } from '@testing-library/react';
import { Provider, createStore } from 'jotai';
import { describe, expect, it } from 'vitest';
import { continuousColor } from '@quent/utils';
import { nodeColoringAtom, selectedScaleTypeAtom } from '../atoms/dagControls';
import { isOperatorGroupSelected, useNodeColoring } from './useNodeColoring';

describe('isOperatorGroupSelected', () => {
  it('deselects a parent visually when any covered child is deselected', () => {
    expect(isOperatorGroupSelected(new Set(['parent', 'right']), 'parent', ['left', 'right'])).toBe(
      false
    );
  });

  it('selects a parent visually when all covered children are selected', () => {
    expect(
      isOperatorGroupSelected(new Set(['parent', 'left', 'right']), 'parent', ['left', 'right'])
    ).toBe(true);
  });
});

describe('useNodeColoring scale', () => {
  // Values 0, 3, 15 and 63: log starts at the smallest positive value (3), linear at 0.
  const coloring = {
    type: 'continuous' as const,
    values: new Map([
      ['zero', 0],
      ['small', 3],
      ['mid', 15],
      ['large', 63],
    ]),
    min: 0,
    logMin: 3,
    max: 63,
  };

  function colorOf(operatorId: string, scale: 'log' | 'linear') {
    const store = createStore();
    store.set(nodeColoringAtom, coloring);
    store.set(selectedScaleTypeAtom, scale);
    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(Provider, { store }, children);
    return renderHook(() => useNodeColoring(operatorId, false), { wrapper }).result.current
      .fieldColor;
  }

  it('log: starts at the smallest positive value and puts 15 at the midpoint', () => {
    expect(colorOf('zero', 'log')).toBe(continuousColor(0, 'blue', false));
    expect(colorOf('small', 'log')).toBe(continuousColor(0, 'blue', false));
    expect(colorOf('mid', 'log')).toBe(continuousColor(0.5, 'blue', false));
    expect(colorOf('large', 'log')).toBe(continuousColor(1, 'blue', false));
  });

  it('linear: keeps proportions from the true minimum', () => {
    expect(colorOf('small', 'linear')).toBe(continuousColor(3 / 63, 'blue', false));
    expect(colorOf('mid', 'linear')).toBe(continuousColor(15 / 63, 'blue', false));
  });
});
