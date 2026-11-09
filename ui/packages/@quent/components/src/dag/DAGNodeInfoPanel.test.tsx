// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useState } from 'react';
import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { createStore, Provider } from 'jotai';
import { useDagDetailAttention, useOperatorSelectionActions } from '@quent/hooks';
import { getDeterministicColor } from '@quent/utils';
import type { DAGEdge, Operator } from '@quent/utils';
import {
  PipeInspectionProbe,
  createPipeInspectionRecorder,
} from '../test-utils/PipeInspectionProbe';
import { DAGNodeInfoPanel } from './DAGNodeInfoPanel';

const hierarchyOperators: Operator[] = [
  {
    id: 'logical',
    plan_id: 'logical-plan',
    parent_operator_ids: [],
    instance_name: 'Logical join',
    operator_type_name: 'LogicalJoin',
    custom_attributes: [],
    statistics: null,
    active_span: null,
  },
  ...[
    ['physical-1', 'Build hash table', 'HashBuild'],
    ['physical-2', 'Probe hash table', 'HashProbe'],
  ].map(([id, instanceName, operatorType]): Operator => ({
    id,
    plan_id: 'physical-plan',
    parent_operator_ids: ['logical'],
    instance_name: instanceName,
    operator_type_name: operatorType,
    custom_attributes: [],
    statistics: null,
    active_span: null,
  })),
];

function SelectedNode({ onExpandedChange }: { onExpandedChange?: (expanded: boolean) => void }) {
  const updateOperatorSelection = useOperatorSelectionActions();

  useEffect(() => {
    updateOperatorSelection({
      type: 'add',
      selectionId: 'logical',
      label: 'Logical join',
      operatorIds: ['logical', 'physical-1', 'physical-2'],
      selectedData: {
        nodeId: 'logical',
        label: 'Logical join',
        operationType: 'logicaljoin',
        statistics: [{ key: 'logical_rows', value: 10 }],
        relatedOperators: [
          {
            nodeId: 'physical-1',
            label: 'Build hash table',
            operationType: 'hashbuild',
            statistics: [{ key: 'build_rows', value: 20 }],
          },
          {
            nodeId: 'physical-2',
            label: 'Probe hash table',
            operationType: 'hashprobe',
            statistics: [{ key: 'probe_rows', value: 30 }],
          },
        ],
      },
    });
  }, [updateOperatorSelection]);

  return <DAGNodeInfoPanel operators={hierarchyOperators} onExpandedChange={onExpandedChange} />;
}

function SameNameOnDifferentWorkers() {
  const updateOperatorSelection = useOperatorSelectionActions();

  useEffect(() => {
    updateOperatorSelection({
      type: 'add',
      selectionId: 'scan',
      label: 'Table scan',
      operatorIds: ['scan', 'scan-1', 'scan-2'],
      selectedData: {
        nodeId: 'scan-1',
        label: 'Table scan',
        operationType: 'scan',
        statistics: [{ key: 'output_rows', value: 10 }],
        workerLabel: 'worker-1',
        planLabel: 'Physical',
        relatedOperators: [
          {
            nodeId: 'scan-2',
            label: 'Table scan',
            operationType: 'scan',
            statistics: [{ key: 'output_rows', value: 20 }],
            workerLabel: 'worker-2',
            planLabel: 'Physical',
          },
        ],
      },
    });
  }, [updateOperatorSelection]);

  return <DAGNodeInfoPanel />;
}

function SwitchSelectedNode() {
  const [showLogical, setShowLogical] = useState(true);
  const updateOperatorSelection = useOperatorSelectionActions();

  useEffect(() => {
    const data = showLogical
      ? {
          nodeId: 'logical',
          label: 'Logical join',
          operationType: 'logicaljoin',
          statistics: [],
        }
      : {
          nodeId: 'scan',
          label: 'Table scan',
          operationType: 'scan',
          statistics: [],
        };
    updateOperatorSelection({
      type: 'replace',
      selections: [
        {
          selectionId: data.nodeId,
          label: data.label,
          operatorIds: [data.nodeId],
          selectedData: data,
        },
      ],
    });
  }, [showLogical, updateOperatorSelection]);

  return (
    <>
      <button onClick={() => setShowLogical(value => !value)}>Switch operator</button>
      <DAGNodeInfoPanel />
    </>
  );
}

function TwoSelectedNodes() {
  const updateOperatorSelection = useOperatorSelectionActions();

  useEffect(() => {
    updateOperatorSelection({
      type: 'add',
      selectionId: 'scan',
      label: 'Table scan',
      operatorIds: ['scan'],
      selectedData: {
        nodeId: 'scan',
        label: 'Table scan',
        operationType: 'scan',
        statistics: [{ key: 'output_rows', value: 10 }],
      },
    });
    updateOperatorSelection({
      type: 'add',
      selectionId: 'join',
      label: 'Hash join',
      operatorIds: ['join', 'probe'],
      selectedData: {
        nodeId: 'join',
        label: 'Hash join',
        operationType: 'hashjoin',
        statistics: [{ key: 'build_rows', value: 20 }],
        relatedOperators: [
          {
            nodeId: 'probe',
            label: 'Probe hash table',
            operationType: 'hashprobe',
            statistics: [{ key: 'probe_rows', value: 30 }],
          },
        ],
      },
    });
  }, [updateOperatorSelection]);

  return <DAGNodeInfoPanel />;
}

function FilterOnly() {
  const updateFilter = useOperatorSelectionActions();
  useEffect(() => {
    updateFilter({
      type: 'add',
      selectionId: 'scan',
      label: 'Table scan',
      operatorIds: ['scan'],
      selectedData: {
        nodeId: 'scan',
        label: 'Table scan',
        operationType: 'scan',
        statistics: [],
      },
    });
  }, [updateFilter]);
  return <DAGNodeInfoPanel />;
}

function LogicalPlanOnly() {
  const updateFilter = useOperatorSelectionActions();
  useEffect(() => {
    updateFilter({
      type: 'add',
      selectionId: 'logical',
      label: 'Logical join',
      operatorIds: ['logical'],
      selectedData: {
        nodeId: 'logical',
        label: 'Logical join',
        operationType: 'logicaljoin',
        statistics: [],
        planLabel: 'Logical',
      },
    });
  }, [updateFilter]);
  return <DAGNodeInfoPanel />;
}

function FilterWithoutDetails({
  onExpandedChange,
}: {
  onExpandedChange?: (expanded: boolean) => void;
}) {
  const updateFilter = useOperatorSelectionActions();
  const attention = useDagDetailAttention();
  useEffect(() => {
    updateFilter({
      type: 'replace',
      selections: [{ selectionId: 'missing', label: 'Missing', operatorIds: ['missing'] }],
    });
  }, [updateFilter]);
  return (
    <>
      <output data-testid="detail-attention">{attention?.id ?? 'none'}</output>
      <DAGNodeInfoPanel onExpandedChange={onExpandedChange} />
    </>
  );
}

describe('DAGNodeInfoPanel', () => {
  it('scrolls to a newly filtered operator', async () => {
    const scrollIntoView = vi.mocked(Element.prototype.scrollIntoView);
    scrollIntoView.mockClear();

    render(
      <Provider>
        <SelectedNode />
      </Provider>
    );

    const entry = await screen.findByTestId('operator-detail-entry-logical');
    expect(scrollIntoView.mock.contexts[scrollIntoView.mock.contexts.length - 1]).toBe(entry);
    expect(scrollIntoView).toHaveBeenLastCalledWith({
      behavior: 'smooth',
      block: 'nearest',
    });
  });

  it('shows filtered operators in details by default', () => {
    render(
      <Provider>
        <FilterOnly />
      </Provider>
    );
    expect(screen.getByTestId('operator-accordion-scan')).toBeVisible();
    expect(screen.getByRole('button', { name: 'Toggle details' })).toBeEnabled();
  });

  it('reports expansion changes to its layout container', async () => {
    const onExpandedChange = vi.fn();
    render(
      <Provider>
        <SelectedNode onExpandedChange={onExpandedChange} />
      </Provider>
    );

    await screen.findByTestId('operator-accordion-logical');
    expect(onExpandedChange).toHaveBeenLastCalledWith(true);

    fireEvent.click(screen.getByRole('button', { name: 'Toggle details' }));

    expect(onExpandedChange).toHaveBeenLastCalledWith(false);
  });

  it('shows statistics for every related child operator', async () => {
    render(
      <Provider>
        <SelectedNode />
      </Provider>
    );

    const accordion = await screen.findByTestId('operator-accordion-logical');
    expect(within(accordion).getByText('Logical join')).toHaveAttribute('title', 'Logical join');
    expect(screen.getByText('Build hash table')).toBeInTheDocument();
    expect(screen.getByText('Probe hash table')).toBeInTheDocument();
    expect(screen.getByText('build rows:')).toBeInTheDocument();
    expect(screen.getByText('probe rows:')).toBeInTheDocument();
    expect(screen.getByText('physical-1')).toBeInTheDocument();
    expect(screen.getByText('physical-2')).toBeInTheDocument();

    const bars = screen.getAllByTestId('operator-color-bar');
    expect(
      bars.filter(bar => bar.getAttribute('data-operation-type') === 'logicaljoin')
    ).not.toHaveLength(0);
    expect(bars.find(bar => bar.getAttribute('data-operation-type') === 'hashbuild')).toHaveStyle({
      backgroundColor: getDeterministicColor('hashbuild'),
    });
    expect(bars.find(bar => bar.getAttribute('data-operation-type') === 'hashprobe')).toHaveStyle({
      backgroundColor: getDeterministicColor('hashprobe'),
    });
  });

  it('removes an operator detail and its filter with one X action', async () => {
    render(
      <Provider>
        <FilterOnly />
      </Provider>
    );

    const removeDetail = await screen.findByRole('button', {
      name: 'Remove operator Table scan from details',
    });

    fireEvent.click(removeDetail);
    expect(screen.queryByTestId('operator-accordion-scan')).not.toBeInTheDocument();
  });

  it('removes a related physical operator while preserving its sibling', async () => {
    render(
      <Provider>
        <SelectedNode />
      </Provider>
    );

    fireEvent.click(
      await screen.findByRole('button', {
        name: 'Remove operator Build hash table from details',
      })
    );

    expect(screen.queryByText('Build hash table')).not.toBeInTheDocument();
    expect(screen.getByText('Probe hash table')).toBeInTheDocument();
    expect(screen.queryByText('Logical join')).not.toBeInTheDocument();
  });

  it('clears detail attention when its operator has no display data', async () => {
    const onExpandedChange = vi.fn();
    render(
      <Provider>
        <FilterWithoutDetails onExpandedChange={onExpandedChange} />
      </Provider>
    );

    await waitFor(() => expect(screen.getByTestId('detail-attention')).toHaveTextContent('none'));
    expect(onExpandedChange).not.toHaveBeenCalledWith(true);
  });

  it('shows statistics for every selected operator', async () => {
    render(
      <Provider>
        <TwoSelectedNodes />
      </Provider>
    );

    const scanAccordion = await screen.findByTestId('operator-accordion-scan');
    const joinAccordion = screen.getByTestId('operator-accordion-join');
    expect(within(scanAccordion).getByText('Table scan')).toHaveAttribute('title', 'Table scan');
    expect(within(joinAccordion).getByText('Hash join')).toHaveAttribute('title', 'Hash join');
    expect(screen.getByText('output rows:')).toBeInTheDocument();
    expect(screen.getByText('build rows:')).toBeInTheDocument();
    expect(screen.getByText('Probe hash table')).toBeInTheDocument();
    expect(screen.getByText('probe rows:')).toBeInTheDocument();

    const scanBar = within(scanAccordion).getAllByTestId('operator-color-bar')[0];
    expect(scanBar).toHaveAttribute('data-operation-type', 'scan');
    expect(scanBar).toHaveStyle({ backgroundColor: getDeterministicColor('scan') });
    const joinBar = within(joinAccordion).getAllByTestId('operator-color-bar')[0];
    expect(joinBar).toHaveAttribute('data-operation-type', 'hashjoin');
    expect(joinBar).toHaveStyle({ backgroundColor: getDeterministicColor('hashjoin') });
  });

  it('collapses a selected operator without hiding the others', async () => {
    render(
      <Provider>
        <TwoSelectedNodes />
      </Provider>
    );

    const scanToggle = await screen.findByRole('button', { name: 'Toggle Table scan details' });
    const joinToggle = screen.getByRole('button', { name: 'Toggle Hash join details' });
    expect(scanToggle).toHaveAttribute('aria-expanded', 'true');
    expect(joinToggle).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByText('output rows:')).toBeInTheDocument();
    expect(screen.getByText('build rows:')).toBeInTheDocument();

    fireEvent.click(scanToggle);

    expect(scanToggle).toHaveAttribute('aria-expanded', 'false');
    expect(joinToggle).toHaveAttribute('aria-expanded', 'true');
    expect(screen.queryByText('output rows:')).not.toBeInTheDocument();
    expect(screen.getByText('build rows:')).toBeInTheDocument();
    expect(screen.getByText('Probe hash table')).toBeInTheDocument();
  });

  it('resets collapsed operators when the selection changes', async () => {
    render(
      <Provider>
        <SwitchSelectedNode />
      </Provider>
    );

    const logicalToggle = await screen.findByRole('button', {
      name: 'Toggle Logical join details',
    });
    fireEvent.click(logicalToggle);
    expect(logicalToggle).toHaveAttribute('aria-expanded', 'false');

    fireEvent.click(screen.getByRole('button', { name: 'Switch operator' }));
    expect(
      await screen.findByRole('button', { name: 'Toggle Table scan details' })
    ).toHaveAttribute('aria-expanded', 'true');

    fireEvent.click(screen.getByRole('button', { name: 'Switch operator' }));
    expect(
      await screen.findByRole('button', { name: 'Toggle Logical join details' })
    ).toHaveAttribute('aria-expanded', 'true');
  });

  it('shows a worker label so same-named operators can be told apart', async () => {
    render(
      <Provider>
        <SameNameOnDifferentWorkers />
      </Provider>
    );

    await screen.findAllByText('Table scan');
    expect(screen.getByText('worker-1')).toBeInTheDocument();
    expect(screen.getByText('worker-2')).toBeInTheDocument();
    expect(screen.queryByText('Physical')).not.toBeInTheDocument();

    // Each toggle must have a distinct accessible name, otherwise screen readers
    // and role-based queries can't tell same-named operators apart.
    expect(
      screen.getByRole('button', { name: 'Toggle Table scan (worker-1) details' })
    ).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: 'Toggle Table scan (worker-2) details' })
    ).toBeInTheDocument();
  });

  it('shows the plan when a logical operator has no worker', async () => {
    render(
      <Provider>
        <LogicalPlanOnly />
      </Provider>
    );

    expect(await screen.findByText('Logical')).toBeVisible();
    expect(
      screen.getByRole('button', {
        name: 'Toggle Logical join (Logical) details',
      })
    ).toBeInTheDocument();
  });

  it('collapses related child operators independently', async () => {
    render(
      <Provider>
        <SelectedNode />
      </Provider>
    );

    const relatedToggle = await screen.findByRole('button', {
      name: 'Toggle Build hash table details',
    });
    expect(relatedToggle).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByText('build rows:')).toBeInTheDocument();

    fireEvent.click(relatedToggle);

    expect(relatedToggle).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByText('build rows:')).not.toBeInTheDocument();
    expect(screen.getByText('probe rows:')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Toggle Logical join details' })).toHaveAttribute(
      'aria-expanded',
      'true'
    );
  });
});

it('shows nested sending and receiving evidence for an inspected pipe without selecting operators', () => {
  const probe = createPipeInspectionRecorder();
  const store = createStore();
  const edges: DAGEdge[] = [
    {
      id: 'pipe',
      source: 'scan',
      target: 'join',
      sourcePortId: 'out',
      targetPortId: 'in',
      portStats: [
        { key: 'Volume', value: { kind: 'struct', fields: [{ key: 'bytes', value: 1024 }] } },
      ],
      targetPortStats: [
        { key: 'Volume', value: { kind: 'struct', fields: [{ key: 'bytes', value: 512 }] } },
      ],
    },
  ];
  render(
    <Provider store={store}>
      <PipeInspectionProbe edges={edges} recorder={probe}>
        <DAGNodeInfoPanel />
      </PipeInspectionProbe>
    </Provider>
  );
  act(() => probe.latest().addInspectedPipe({ sourcePortId: 'out', targetPortId: 'in' }));
  expect(screen.getByText('Details')).toBeVisible();
  expect(
    within(screen.getByRole('region', { name: 'Sending port' })).getByText('1.00 KiB')
  ).toBeVisible();
  expect(
    within(screen.getByRole('region', { name: 'Receiving port' })).getByText('512.00 B')
  ).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Toggle details' }));
  expect(screen.queryByRole('region', { name: 'Sending port' })).not.toBeInTheDocument();
});

describe('pipes alongside operators', () => {
  const edges = [
    { id: 'one', source: 'scan', target: 'join', sourcePortId: 'out-1', targetPortId: 'in-1' },
    { id: 'two', source: 'scan', target: 'join', sourcePortId: 'out-2', targetPortId: 'in-2' },
  ];
  const renderPanel = () => {
    const probe = createPipeInspectionRecorder();
    render(
      <Provider store={createStore()}>
        <PipeInspectionProbe edges={edges} recorder={probe}>
          <DAGNodeInfoPanel />
        </PipeInspectionProbe>
      </Provider>
    );
    return probe;
  };
  const selectOperator = (probe: ReturnType<typeof createPipeInspectionRecorder>) =>
    act(() =>
      probe.latest().updateSelection({
        type: 'add',
        selectionId: 'scan',
        label: 'Table scan',
        operatorIds: ['scan'],
        selectedData: {
          nodeId: 'scan',
          label: 'Table scan',
          operationType: 'scan',
          statistics: [],
        },
      })
    );

  it('offers a Pipes tab next to the operator tab', async () => {
    const probe = renderPanel();
    selectOperator(probe);
    expect(screen.queryByRole('tab', { name: 'Pipes' })).not.toBeInTheDocument();

    const scrollIntoView = vi.mocked(Element.prototype.scrollIntoView);
    scrollIntoView.mockClear();
    act(() => probe.latest().addInspectedPipe({ sourcePortId: 'out-1', targetPortId: 'in-1' }));
    expect(screen.getByRole('tab', { name: 'Operators' })).toBeVisible();
    expect(screen.getByRole('tab', { name: 'Pipes' })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('region', { name: 'Sending port' })).toBeVisible();
    const pipeEntry = screen.getByTestId('pipe-detail-entry-one');
    expect(scrollIntoView.mock.contexts[scrollIntoView.mock.contexts.length - 1]).toBe(pipeEntry);

    fireEvent.mouseDown(screen.getByRole('tab', { name: 'Operators' }), { button: 0 });
    expect(screen.getByRole('tab', { name: 'Operators' })).toHaveAttribute('aria-selected', 'true');
    expect(probe.latest().selectedOperatorIds).toEqual(new Set(['scan']));
    expect(probe.latest().inspectedPipes).toHaveLength(1);
  });

  it('switches to and scrolls to a newly filtered operator', async () => {
    const probe = renderPanel();
    act(() => probe.latest().addInspectedPipe({ sourcePortId: 'out-1', targetPortId: 'in-1' }));

    const scrollIntoView = vi.mocked(Element.prototype.scrollIntoView);
    scrollIntoView.mockClear();
    act(() => {
      probe.latest().updateSelection({
        type: 'add',
        selectionId: 'scan',
        label: 'Table scan',
        operatorIds: ['scan'],
        selectedData: {
          nodeId: 'scan',
          label: 'Table scan',
          operationType: 'scan',
          statistics: [],
        },
      });
    });

    expect(screen.getByRole('tab', { name: 'Operators' })).toHaveAttribute('aria-selected', 'true');
    const operatorEntry = screen.getByTestId('operator-detail-entry-scan');
    await waitFor(() =>
      expect(scrollIntoView.mock.contexts[scrollIntoView.mock.contexts.length - 1]).toBe(
        operatorEntry
      )
    );
  });

  it('keeps the Pipes tab active when filtering an endpoint from pipe details', () => {
    const probe = createPipeInspectionRecorder();
    const nodes = [
      { id: 'scan', label: 'Table scan', type: 'scan' },
      { id: 'join', label: 'Hash join', type: 'join' },
    ];
    render(
      <Provider store={createStore()}>
        <PipeInspectionProbe edges={edges} nodes={nodes} recorder={probe}>
          <DAGNodeInfoPanel />
        </PipeInspectionProbe>
      </Provider>
    );
    act(() => probe.latest().addInspectedPipe({ sourcePortId: 'out-1', targetPortId: 'in-1' }));

    const scrollIntoView = vi.mocked(Element.prototype.scrollIntoView);
    scrollIntoView.mockClear();
    fireEvent.click(screen.getByRole('button', { name: 'Filter by Table scan operator' }));

    expect(screen.getByRole('tab', { name: 'Pipes' })).toHaveAttribute('aria-selected', 'true');
    expect(probe.latest().selectedOperatorIds).toEqual(new Set(['scan']));
    expect(scrollIntoView).not.toHaveBeenCalled();
  });

  it('stacks each further pipe below the first', () => {
    const probe = renderPanel();
    act(() => probe.latest().addInspectedPipe({ sourcePortId: 'out-1', targetPortId: 'in-1' }));
    expect(screen.getAllByRole('region', { name: 'Sending port' })).toHaveLength(1);
    act(() => probe.latest().addInspectedPipe({ sourcePortId: 'out-2', targetPortId: 'in-2' }));
    expect(screen.getAllByRole('region', { name: 'Sending port' })).toHaveLength(2);
    expect(screen.getAllByRole('region', { name: 'Receiving port' })).toHaveLength(2);
  });

  it('removes a pipe from its detail entry without offering a pipe filter', () => {
    const probe = renderPanel();
    act(() => probe.latest().addInspectedPipe({ sourcePortId: 'out-1', targetPortId: 'in-1' }));

    expect(screen.queryByRole('button', { name: /filter pipe/i })).not.toBeInTheDocument();
    const removePipe = screen.getByRole('button', {
      name: 'Remove pipe from scan to join from details',
    });
    expect(removePipe).toHaveClass('text-foreground');
    fireEvent.click(removePipe);

    expect(probe.latest().inspectedPipes).toHaveLength(0);
    expect(screen.queryByRole('tab', { name: 'Pipes' })).not.toBeInTheDocument();
  });
});
