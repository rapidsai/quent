// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { type ComponentType, type ReactNode } from 'react';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { Provider, createStore } from 'jotai';
import { describe, expect, it, vi } from 'vitest';
import type { Edge, EdgeProps } from '@xyflow/react';
import {
  useDagEdgeColoring,
  useDagEdgeWidthConfig,
  useEdgeColorPalette,
  useSelectedEdgeColorField,
  useSelectedScaleType,
  useSelectedEdgeWidthField,
} from '@quent/hooks';
import { continuousColor } from '@quent/utils';
import type { DAGData } from '../services/query-plan/types';
import {
  computeEdgeColoring,
  computeEdgeWidthConfig,
} from '../services/query-plan/dagFieldProcessing';
import { DAGChart } from './DAGChart';

const mocks = vi.hoisted(() => ({ fitView: vi.fn() }));

// Replace layout and the canvas host; render the actual registered edge components
// and legend, driven by the real statistic-processing and selection hooks.
vi.mock('./layout', async importOriginal => ({
  ...(await importOriginal<typeof import('./layout')>()),
  calculateLayout: async (nodes: unknown[], edges: unknown[]) => ({ nodes, edges }),
}));
vi.mock('@xyflow/react', async importOriginal => {
  const actual = await importOriginal<typeof import('@xyflow/react')>();
  return {
    ...actual,
    useReactFlow: () => ({ fitView: mocks.fitView }),
    Background: () => null,
    MiniMap: () => null,
    Panel: ({ children }: { children: ReactNode }) => <div>{children}</div>,
    ReactFlow: ({
      edges,
      edgeTypes,
      children,
    }: {
      edges: Edge[];
      edgeTypes: Record<string, ComponentType<EdgeProps>>;
      children: ReactNode;
    }) => (
      <div>
        <svg>
          {edges.map(edge => {
            const EdgeComponent = edgeTypes[edge.type ?? 'default'];
            return (
              <EdgeComponent
                key={edge.id}
                id={edge.id}
                source={edge.source}
                target={edge.target}
                sourceX={0}
                sourceY={0}
                targetX={0}
                targetY={100}
                sourcePosition={actual.Position.Bottom}
                targetPosition={actual.Position.Top}
                data={edge.data}
              />
            );
          })}
        </svg>
        {children}
      </div>
    ),
  };
});

const data: DAGData = {
  nodes: [],
  edges: [0, 3, 15, 63].map((value, index) => ({
    id: `edge-${index}`,
    source: `node-${index}`,
    target: `node-${index + 1}`,
    portStats: [{ key: 'bytes', value }],
  })),
  queryData: [],
};

function EdgeConfiguration() {
  const [, setColorField] = useSelectedEdgeColorField();
  const [, setWidthField] = useSelectedEdgeWidthField();
  const [, setPalette] = useEdgeColorPalette();
  const [, setScaleType] = useSelectedScaleType();
  useDagEdgeColoring(data.edges, computeEdgeColoring);
  useDagEdgeWidthConfig(data.edges, computeEdgeWidthConfig);
  return (
    <>
      <button
        onClick={() => {
          setColorField('bytes');
          setWidthField('bytes');
          setPalette('blue');
        }}
      >
        Select bytes
      </button>
      <button onClick={() => setScaleType('linear')}>Use linear scale</button>
    </>
  );
}

describe('DAGChart edge scaling', () => {
  it('renders width and color at the same logarithmic positions and identifies the legend scale', async () => {
    const { container } = render(
      <Provider store={createStore()}>
        <EdgeConfiguration />
        <DAGChart data={data} isDark={false} />
      </Provider>
    );

    fireEvent.click(screen.getByRole('button', { name: 'Select bytes' }));
    await waitFor(() => {
      expect(container.querySelector('#edge-2')).toHaveStyle({
        stroke: continuousColor(0.5, 'blue'),
      });
    });
    expect(
      Number.parseFloat((container.querySelector('#edge-2') as SVGElement).style.strokeWidth)
    ).toBeCloseTo(13.5, 6);
    // The scale starts at the smallest positive value, so a zero and the
    // smallest positive edge both sit at the bottom of the scale.
    for (const id of ['#edge-0', '#edge-1']) {
      expect(container.querySelector(id)).toHaveStyle({
        strokeWidth: '2',
        stroke: continuousColor(0, 'blue'),
      });
    }
    expect(container.querySelector('#edge-3')).toHaveStyle({
      strokeWidth: '25',
      stroke: continuousColor(1, 'blue'),
    });
    // The scale type is a badge beside the title, and the legend labels the log midpoint.
    expect(screen.getByText('bytes')).toBeVisible();
    expect(screen.getByText('Log')).toBeVisible();
    expect(screen.getByText('15.00 B')).toBeVisible();
    expect(screen.getByText('0 B')).toBeVisible();
    // The legend shows the true data range, including the zero edge.
    expect(screen.getByText('63.00 B')).toBeVisible();
  });

  it('switches width, color and legend to a linear scale', async () => {
    const { container } = render(
      <Provider store={createStore()}>
        <EdgeConfiguration />
        <DAGChart data={data} isDark={false} />
      </Provider>
    );

    fireEvent.click(screen.getByRole('button', { name: 'Select bytes' }));
    fireEvent.click(screen.getByRole('button', { name: 'Use linear scale' }));

    // 15 of a 0..63 range sits at 15/63 along the scale.
    const t = 15 / 63;
    await waitFor(() => {
      expect(container.querySelector('#edge-2')).toHaveStyle({
        stroke: continuousColor(t, 'blue'),
      });
    });
    expect(
      Number.parseFloat((container.querySelector('#edge-2') as SVGElement).style.strokeWidth)
    ).toBeCloseTo(2 + t * 23, 6);
    // Linear starts at the true minimum, so the smallest positive edge is no longer at the bottom.
    expect(
      Number.parseFloat((container.querySelector('#edge-1') as SVGElement).style.strokeWidth)
    ).toBeGreaterThan(2);
    expect(screen.getByText('Linear')).toBeVisible();
    expect(screen.queryByText('Log')).toBeNull();
    expect(screen.getByText('31.50 B')).toBeVisible();
  });
});
