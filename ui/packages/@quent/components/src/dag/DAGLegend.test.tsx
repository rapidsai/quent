// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { type ReactNode } from 'react';
import { act, render, screen } from '@testing-library/react';
import { Provider, createStore } from 'jotai';
import { describe, expect, it, vi } from 'vitest';
import { useSelectedColorField, useSelectedScaleType, useSetNodeColoring } from '@quent/hooks';
import { DAGLegend } from './DAGLegend';

vi.mock('@xyflow/react', () => ({
  Panel: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));

// Values 0, 3, 15 and 63: the log scale starts at 3, the linear scale at 0.
const coloring = {
  type: 'continuous' as const,
  values: new Map([
    ['a', 0],
    ['b', 3],
    ['c', 15],
    ['d', 63],
  ]),
  min: 0,
  logMin: 3,
  max: 63,
};

function NodeColoringSetup() {
  const setNodeColoring = useSetNodeColoring();
  const [, setColorField] = useSelectedColorField();
  const [, setScaleType] = useSelectedScaleType();
  return (
    <>
      <button
        onClick={() => {
          setNodeColoring(coloring);
          setColorField('bytes');
        }}
      >
        Color nodes
      </button>
      <button onClick={() => setScaleType('linear')}>Use linear scale</button>
    </>
  );
}

function renderLegend() {
  render(
    <Provider store={createStore()}>
      <NodeColoringSetup />
      <DAGLegend isDark={false} />
    </Provider>
  );
  act(() => screen.getByRole('button', { name: 'Color nodes' }).click());
}

describe('DAGLegend node coloring scale', () => {
  it('shows a Log badge and the log midpoint by default', () => {
    renderLegend();
    expect(screen.getByText('bytes')).toBeVisible();
    expect(screen.getByText('Log')).toBeVisible();
    expect(screen.getByText('0 B')).toBeVisible();
    expect(screen.getByText('15.00 B')).toBeVisible();
    expect(screen.getByText('63.00 B')).toBeVisible();
  });

  it('shows a Linear badge and the arithmetic midpoint after switching', () => {
    renderLegend();
    act(() => screen.getByRole('button', { name: 'Use linear scale' }).click());
    expect(screen.getByText('Linear')).toBeVisible();
    expect(screen.queryByText('Log')).toBeNull();
    expect(screen.getByText('31.50 B')).toBeVisible();
  });
});
