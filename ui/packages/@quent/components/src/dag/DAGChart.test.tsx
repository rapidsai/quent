// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  isValidElement,
  type ComponentType,
  type MouseEvent as ReactMouseEvent,
  type ReactNode,
} from 'react';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { Provider, createStore } from 'jotai';
import { describe, expect, it, vi } from 'vitest';
import type { Edge, EdgeProps, Node } from '@xyflow/react';
import {
  useDagEdgeColoring,
  useDagEdgeWidthConfig,
  useEdgeColorPalette,
  useInspectedPipes,
  usePinnedOperatorDetailIds,
  useSelectedOperatorIds,
  useSelectedEdgeColorField,
  useSelectedScaleType,
  useSelectedEdgeWidthField,
} from '@quent/hooks';
import { continuousColor, flattenStatistics } from '@quent/utils';
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
    EdgeLabelRenderer: ({ children }: { children: ReactNode }) =>
      isValidElement<{ className?: string }>(children) &&
      children.props.className?.includes('nodrag')
        ? children
        : null,
    Panel: ({ children }: { children: ReactNode }) => <div>{children}</div>,
    ReactFlow: ({
      nodes = [],
      edges,
      edgeTypes,
      children,
      onNodeClick,
      onSelectionStart,
      onSelectionChange,
      onSelectionEnd,
      onPaneClick,
      selectionKeyCode,
      multiSelectionKeyCode,
    }: {
      nodes?: Node[];
      edges: Edge[];
      edgeTypes: Record<string, ComponentType<EdgeProps>>;
      children: ReactNode;
      onNodeClick?: (event: ReactMouseEvent, node: Node) => void;
      onSelectionStart?: () => void;
      onSelectionChange?: (selection: { nodes: Node[]; edges: Edge[] }) => void;
      onSelectionEnd?: () => void;
      onPaneClick?: () => void;
      selectionKeyCode?: string | null;
      multiSelectionKeyCode?: string | null;
    }) => (
      <div
        data-testid="flow"
        data-selection-key={selectionKeyCode}
        data-multi-selection-key={multiSelectionKeyCode}
      >
        {nodes.map(node => (
          <button
            key={node.id}
            type="button"
            onClick={event => onNodeClick?.(event, node)}
          >{`Select ${node.id}`}</button>
        ))}
        <button
          type="button"
          onClick={() => {
            onSelectionStart?.();
            onSelectionChange?.({ nodes, edges });
            onSelectionEnd?.();
          }}
        >
          Box select all
        </button>
        <button type="button" onClick={onPaneClick}>
          Click DAG background
        </button>
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
  edges: [0, 3, 15, 63].map((value, index) => {
    const portStats = [{ key: 'bytes', value }];
    return {
      id: `edge-${index}`,
      source: `node-${index}`,
      target: `node-${index + 1}`,
      portStats,
      statisticFields: flattenStatistics(portStats),
    };
  }),
  queryData: [],
};

const selectionData: DAGData = {
  nodes: [
    { id: 'first', label: 'First', type: 'scan' },
    { id: 'second', label: 'Second', type: 'filter' },
  ],
  edges: [],
  queryData: [],
};

const pipeData: DAGData = {
  nodes: [
    { id: 'source', label: 'Source', type: 'scan' },
    { id: 'middle', label: 'Middle', type: 'filter' },
    { id: 'target', label: 'Target', type: 'output' },
  ],
  edges: [
    {
      id: 'first-pipe',
      source: 'source',
      target: 'middle',
      sourcePortId: 'source-out',
      targetPortId: 'middle-in',
    },
    {
      id: 'second-pipe',
      source: 'middle',
      target: 'target',
      sourcePortId: 'middle-out',
      targetPortId: 'target-in',
    },
  ],
  queryData: [],
};

const metricPipeData: DAGData = {
  ...pipeData,
  edges: pipeData.edges.map((edge, index) => {
    const portStats = [{ key: 'bytes', value: index + 1 }];
    return {
      ...edge,
      portStats,
      statisticFields: flattenStatistics(portStats),
    };
  }),
};

function SelectionProbe() {
  const pinnedOperatorDetailIds = usePinnedOperatorDetailIds();
  const filteredOperatorIds = useSelectedOperatorIds();
  const inspectedPipes = useInspectedPipes();
  return (
    <>
      <span data-testid="selected-operators">{[...pinnedOperatorDetailIds].sort().join(',')}</span>
      <span data-testid="filtered-operators">{[...filteredOperatorIds].sort().join(',')}</span>
      <span data-testid="selected-pipes">
        {inspectedPipes
          .map(pipe => pipe.id)
          .sort()
          .join(',')}
      </span>
    </>
  );
}

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

function PipeWidthConfiguration() {
  const [, setWidthField] = useSelectedEdgeWidthField();
  useDagEdgeWidthConfig(metricPipeData.edges, computeEdgeWidthConfig);
  return <button onClick={() => setWidthField('bytes')}>Select pipe width</button>;
}

function translateY(element: HTMLElement): number {
  const match = element.style.transform.match(/translate\([^,]+px,\s*([^)]+)px\)$/);
  if (!match) {
    throw new Error(`Missing pixel translation in "${element.style.transform}"`);
  }
  return Number(match[1]);
}

describe('DAGChart selection', () => {
  it('adds and removes node pins on click', async () => {
    render(
      <Provider store={createStore()}>
        <SelectionProbe />
        <DAGChart data={selectionData} isDark={false} />
      </Provider>
    );

    const first = await screen.findByRole('button', { name: 'Select first' });
    const second = screen.getByRole('button', { name: 'Select second' });

    fireEvent.click(first);
    expect(screen.getByTestId('selected-operators')).toHaveTextContent('first');
    expect(screen.getByTestId('filtered-operators')).toBeEmptyDOMElement();
    fireEvent.click(first);
    expect(screen.getByTestId('selected-operators')).toBeEmptyDOMElement();

    fireEvent.click(second);
    expect(screen.getByTestId('selected-operators')).toHaveTextContent('second');

    fireEvent.click(first);
    expect(screen.getByTestId('selected-operators')).toHaveTextContent('first,second');
    fireEvent.click(second);
    expect(screen.getByTestId('selected-operators')).toHaveTextContent('first');
    fireEvent.click(first);
    expect(screen.getByTestId('selected-operators')).toBeEmptyDOMElement();
  });

  it('disables modifier-based box selection', async () => {
    render(
      <Provider store={createStore()}>
        <SelectionProbe />
        <DAGChart data={pipeData} isDark={false} />
      </Provider>
    );

    const flow = screen.getByTestId('flow');
    expect(flow).not.toHaveAttribute('data-selection-key');
    expect(flow).not.toHaveAttribute('data-multi-selection-key');

    fireEvent.click(await screen.findByRole('button', { name: 'Box select all' }));
    expect(screen.getByTestId('selected-operators')).toBeEmptyDOMElement();
    expect(screen.getByTestId('selected-pipes')).toBeEmptyDOMElement();
  });

  it('adds and removes pipe pins on click', async () => {
    render(
      <Provider store={createStore()}>
        <SelectionProbe />
        <DAGChart data={pipeData} isDark={false} />
      </Provider>
    );

    fireEvent.click(await screen.findByRole('button', { name: 'Select source' }));
    expect(screen.getByTestId('selected-operators')).toHaveTextContent('source');
    const pipes = await screen.findAllByRole('button', { name: /^Inspect pipe / });
    fireEvent.click(pipes[0]);
    expect(screen.getByTestId('selected-operators')).toHaveTextContent('source');
    expect(screen.getByTestId('selected-pipes')).toHaveTextContent('first-pipe');
    expect(screen.getByRole('status', { name: 'source-out to middle-in: pinned' })).toBeVisible();
    fireEvent.click(pipes[0]);
    expect(screen.getByTestId('selected-pipes')).toBeEmptyDOMElement();
    expect(
      screen.queryByRole('status', { name: 'source-out to middle-in: pinned' })
    ).not.toBeInTheDocument();

    fireEvent.click(pipes[1]);
    expect(screen.getByTestId('selected-pipes')).toHaveTextContent('second-pipe');

    fireEvent.click(pipes[0]);
    expect(screen.getByTestId('selected-pipes')).toHaveTextContent('first-pipe,second-pipe');
    fireEvent.click(pipes[1]);
    expect(screen.getByTestId('selected-pipes')).toHaveTextContent('first-pipe');
    fireEvent.click(pipes[0]);
    expect(screen.getByTestId('selected-pipes')).toBeEmptyDOMElement();
    expect(screen.getByTestId('selected-operators')).toHaveTextContent('source');
  });

  it('reveals pipe actions when the pipe receives keyboard focus', async () => {
    render(
      <Provider store={createStore()}>
        <DAGChart data={pipeData} isDark={false} />
      </Provider>
    );

    const pipe = (await screen.findAllByRole('button', { name: /^Inspect pipe / }))[0];
    const actions = screen.getByRole('toolbar', { name: 'Actions for source-out to middle-in' });
    expect(actions).toHaveClass('opacity-0', 'overflow-hidden', 'rounded-md');
    for (const button of within(actions).getAllByRole('button')) {
      expect(button).not.toHaveClass('rounded-md');
    }

    fireEvent.focus(pipe);

    expect(actions).toHaveClass('opacity-100');

    const preview = within(actions).getByRole('button', {
      name: 'Preview details for source-out to middle-in',
    });
    fireEvent.focus(preview);
    fireEvent.mouseLeave(actions);

    expect(actions).toHaveClass('opacity-0');
  });

  it('moves pipe actions and status clear of a metric label', async () => {
    render(
      <Provider store={createStore()}>
        <PipeWidthConfiguration />
        <DAGChart data={metricPipeData} isDark={false} />
      </Provider>
    );

    const actions = await screen.findByRole('toolbar', {
      name: 'Actions for source-out to middle-in',
    });
    const actionsContainer = actions.parentElement as HTMLElement;
    const defaultActionsY = translateY(actionsContainer);

    fireEvent.click(screen.getByRole('button', { name: 'Select pipe width' }));
    await waitFor(() => expect(translateY(actionsContainer)).toBeLessThan(defaultActionsY - 7));
    const metricActionsY = translateY(actionsContainer);

    fireEvent.click(screen.getByRole('button', { name: 'Inspect pipe source-out to middle-in' }));
    const status = screen.getByRole('status', { name: 'source-out to middle-in: pinned' });
    expect(translateY(status.parentElement as HTMLElement)).toBeCloseTo(metricActionsY + 48);
  });

  it('closes an edge preview when the DAG background is clicked', async () => {
    render(
      <Provider store={createStore()}>
        <DAGChart data={pipeData} isDark={false} />
      </Provider>
    );

    fireEvent.click(
      await screen.findByRole('button', {
        name: 'Preview details for source-out to middle-in',
      })
    );
    expect(await screen.findByTestId('pipe-details-first-pipe')).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: 'Click DAG background' }));

    await waitFor(() =>
      expect(screen.queryByTestId('pipe-details-first-pipe')).not.toBeInTheDocument()
    );
  });
});

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
    // The scale type is a badge beside the title, and the legend shows only its endpoints.
    expect(screen.getByText('bytes')).toBeVisible();
    expect(screen.getByText('Log')).toBeVisible();
    expect(screen.queryByText('15.00 B')).toBeNull();
    expect(screen.getByText('0 B')).toBeVisible();
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
    expect(screen.queryByText('31.50 B')).toBeNull();
  });
});
