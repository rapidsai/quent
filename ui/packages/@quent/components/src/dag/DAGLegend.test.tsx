// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { type ReactNode } from 'react';
import { act, render, screen } from '@testing-library/react';
import { Provider, createStore } from 'jotai';
import { describe, expect, it, vi } from 'vitest';
import { useSelectedColorField, useSelectedScaleType, useSetNodeColoring } from '@quent/hooks';
import { flattenStatistics } from '@quent/utils';
import { DAGLegend } from './DAGLegend';

vi.mock('@xyflow/react', () => ({
  Panel: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));

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
const nestedField = flattenStatistics([
  {
    key: 'Execution profile',
    value: {
      kind: 'struct',
      fields: [
        {
          key: 'Data volume',
          value: {
            kind: 'struct',
            fields: [
              {
                key: 'Input',
                value: {
                  kind: 'struct',
                  fields: [{ key: 'batches', value: 5115 }],
                },
              },
            ],
          },
        },
      ],
    },
  },
])[0];

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
  it('shows only the minimum and maximum values', () => {
    renderLegend();
    expect(screen.getByText('bytes')).toHaveAttribute('title', 'bytes');
    expect(screen.getByText('Log')).toBeVisible();
    expect(screen.queryByRole('button', { name: 'Log' })).toBeNull();
    expect(screen.getByText('0 B')).toBeVisible();
    expect(screen.getByText('63.00 B')).toBeVisible();
    expect(screen.queryByText('15.00 B')).toBeNull();
  });

  it('updates the passive badge when the scale changes', () => {
    renderLegend();
    act(() => screen.getByRole('button', { name: 'Use linear scale' }).click());
    expect(screen.getByText('Linear')).toBeVisible();
    expect(screen.queryByRole('button', { name: 'Linear' })).toBeNull();
    expect(screen.queryByText('Log')).toBeNull();
    expect(screen.queryByText('31.50 B')).toBeNull();
  });

  it('formats an encoded nested field and exposes the full path as a title', () => {
    function NestedFieldSetup() {
      const setNodeColoring = useSetNodeColoring();
      const [, setColorField] = useSelectedColorField();
      return (
        <button
          onClick={() => {
            setNodeColoring(coloring);
            setColorField(nestedField.key);
          }}
        >
          Color by nested field
        </button>
      );
    }

    render(
      <Provider store={createStore()}>
        <NestedFieldSetup />
        <DAGLegend isDark={false} />
      </Provider>
    );
    act(() => screen.getByRole('button', { name: 'Color by nested field' }).click());

    const label = 'Execution profile › Data volume › Input › batches';
    expect(screen.getByText(label)).toHaveAttribute('title', label);
  });
});
