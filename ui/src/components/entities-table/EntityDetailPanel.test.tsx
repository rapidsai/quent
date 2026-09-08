// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { EntityRef, FiniteStateMachine, QueryBundle } from '@quent/utils';
import { EntityDetailPanel } from './EntityDetailPanel';

// ---------------------------------------------------------------------------
// Mocks
// ---------------------------------------------------------------------------

vi.mock('@/contexts/ThemeContext', () => ({
  useTheme: () => ({ theme: 'light' }),
  THEME_DARK: 'dark',
}));

vi.mock('@quent/components', async importOriginal => {
  const actual = await importOriginal<typeof import('@quent/components')>();
  return {
    ...actual,
    FsmCapacityChart: () => <div data-testid="fsm-capacity-chart" />,
  };
});

vi.mock('./ResourceUsageList', () => ({
  ResourceUsageList: () => null,
}));

vi.mock('./TransitionAttributes', () => ({
  TransitionAttributes: () => null,
}));

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

function makeTransition(name: string, timestamp: number): FiniteStateMachine['transitions'][0] {
  return { name, timestamp, usages: [], attributes: [], derived_attributes: [] };
}

const QUERY_BUNDLE = {
  entities: { resources: {}, resource_types: {} },
  quantity_specs: {},
} as unknown as QueryBundle<EntityRef>;

const BASE_FSM: FiniteStateMachine = {
  id: 'test-uuid-1234',
  instance_name: 'task-7',
  type_name: 'task',
  transitions: [
    makeTransition('queueing', 0),
    makeTransition('running', 0.001),
    makeTransition('done', 0.003),
  ],
};

const DEFAULT_PROPS = {
  resourceLabel: (id: string) => id,
  operatorLabel: (id: string) => id,
  queryBundle: QUERY_BUNDLE,
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('EntityDetailPanel', () => {
  describe('empty state', () => {
    it('shows a placeholder when fsm is null', () => {
      render(<EntityDetailPanel {...DEFAULT_PROPS} fsm={null} />);
      expect(screen.getByText('Select an entity to view its states.')).toBeInTheDocument();
    });

    it('renders nothing structural when fsm is null', () => {
      render(<EntityDetailPanel {...DEFAULT_PROPS} fsm={null} />);
      expect(screen.queryByRole('list')).not.toBeInTheDocument();
    });
  });

  describe('header', () => {
    it('shows the instance name and type badge', () => {
      render(<EntityDetailPanel {...DEFAULT_PROPS} fsm={BASE_FSM} />);
      expect(screen.getByText('task-7')).toBeInTheDocument();
      expect(screen.getByText('task')).toBeInTheDocument();
    });

    it('shows the entity id', () => {
      render(<EntityDetailPanel {...DEFAULT_PROPS} fsm={BASE_FSM} />);
      expect(screen.getByText('test-uuid-1234')).toBeInTheDocument();
    });

    it('copies the entity id when the copy button is clicked', async () => {
      const writeText = vi.fn().mockResolvedValue(undefined);
      Object.assign(navigator, { clipboard: { writeText } });

      render(<EntityDetailPanel {...DEFAULT_PROPS} fsm={BASE_FSM} />);
      fireEvent.click(screen.getByRole('button', { name: 'Copy ID' }));

      expect(writeText).toHaveBeenCalledWith('test-uuid-1234');
    });
  });

  describe('total span', () => {
    it('displays the total span derived from the first and last transition timestamps', () => {
      // timestamps: 0s → 1s → total span = 1000ms
      const fsm: FiniteStateMachine = {
        ...BASE_FSM,
        transitions: [makeTransition('running', 0), makeTransition('done', 1)],
      };
      render(<EntityDetailPanel {...DEFAULT_PROPS} fsm={fsm} />);
      // Scope to the "Total span" row to avoid ambiguity with the transition duration
      const totalSpanRow = screen.getByText('Total span').closest('div');
      expect(totalSpanRow).toHaveTextContent('1.00s');
    });

    it('shows a zero span when there is only one transition', () => {
      const fsm: FiniteStateMachine = {
        ...BASE_FSM,
        transitions: [makeTransition('running', 5)],
      };
      render(<EntityDetailPanel {...DEFAULT_PROPS} fsm={fsm} />);
      // formatDuration(0) returns "0.00ns"
      const totalSpanRow = screen.getByText('Total span').closest('div');
      expect(totalSpanRow).toHaveTextContent('0.00ns');
    });
  });

  describe('dominant state', () => {
    it('shows the state with the most accumulated time', () => {
      // queueing: 1ms, running: 2ms → dominant is running (66.7%)
      const fsm: FiniteStateMachine = {
        ...BASE_FSM,
        transitions: [
          makeTransition('queueing', 0),
          makeTransition('running', 0.001),
          makeTransition('done', 0.003),
        ],
      };
      render(<EntityDetailPanel {...DEFAULT_PROPS} fsm={fsm} />);
      expect(screen.getByText('Dominant state')).toBeInTheDocument();
      // The dominant state name and percentage are rendered together in one element
      expect(screen.getByText(/running.*66\.7%/)).toBeInTheDocument();
    });

    it('does not show dominant state when there is only one transition (no measurable durations)', () => {
      const fsm: FiniteStateMachine = {
        ...BASE_FSM,
        transitions: [makeTransition('running', 0)],
      };
      render(<EntityDetailPanel {...DEFAULT_PROPS} fsm={fsm} />);
      expect(screen.queryByText('Dominant state')).not.toBeInTheDocument();
    });

    it('uses stateColorFn to color the dominant state when provided', () => {
      const stateColorFn = vi.fn().mockReturnValue('#ff0000');
      const fsm: FiniteStateMachine = {
        ...BASE_FSM,
        transitions: [makeTransition('running', 0), makeTransition('done', 1)],
      };
      render(<EntityDetailPanel {...DEFAULT_PROPS} fsm={fsm} stateColorFn={stateColorFn} />);
      expect(stateColorFn).toHaveBeenCalledWith('running');
    });

    it('accumulates time correctly for repeated states', () => {
      // running twice: 1ms + 3ms = 4ms; idle once: 2ms → dominant is running (66.7%)
      const fsm: FiniteStateMachine = {
        ...BASE_FSM,
        transitions: [
          makeTransition('running', 0),
          makeTransition('idle', 0.001),
          makeTransition('running', 0.003),
          makeTransition('done', 0.006),
        ],
      };
      render(<EntityDetailPanel {...DEFAULT_PROPS} fsm={fsm} />);
      // total span 6ms, running = 4ms = 66.7%
      expect(screen.getByText(/running.*66\.7%/)).toBeInTheDocument();
    });
  });

  describe('transition list', () => {
    it('renders all transitions with 1-based indices', () => {
      render(<EntityDetailPanel {...DEFAULT_PROPS} fsm={BASE_FSM} />);
      // The index spans render as "1.", "2.", "3." — scope to span to avoid ambiguity
      expect(screen.getAllByText('1.', { selector: 'span' })).toHaveLength(1);
      expect(screen.getAllByText('2.', { selector: 'span' })).toHaveLength(1);
      expect(screen.getAllByText('3.', { selector: 'span' })).toHaveLength(1);
    });

    it('shows all transition state names', () => {
      render(<EntityDetailPanel {...DEFAULT_PROPS} fsm={BASE_FSM} />);
      expect(screen.getByText('queueing')).toBeInTheDocument();
      expect(screen.getByText('running')).toBeInTheDocument();
      expect(screen.getByText('done')).toBeInTheDocument();
    });

    it('shows a duration for all transitions except the last', () => {
      // transitions at 0ms, 500ms, 1000ms
      const fsm: FiniteStateMachine = {
        ...BASE_FSM,
        transitions: [
          makeTransition('queueing', 0),
          makeTransition('running', 0.5),
          makeTransition('done', 1),
        ],
      };
      render(<EntityDetailPanel {...DEFAULT_PROPS} fsm={fsm} />);
      // Both intermediate transitions have a duration of 500ms
      const durations = screen.getAllByText('500.00ms');
      expect(durations).toHaveLength(2);
    });

    it('highlights a bottleneck transition that consumes more than 50% of total span', () => {
      // running: 900ms out of 1000ms total = 90% → bottleneck
      const fsm: FiniteStateMachine = {
        ...BASE_FSM,
        transitions: [
          makeTransition('running', 0),
          makeTransition('done', 0.9),
          makeTransition('end', 1),
        ],
      };
      render(<EntityDetailPanel {...DEFAULT_PROPS} fsm={fsm} />);
      const bottleneckDuration = screen.getByText('900.00ms');
      expect(bottleneckDuration).toHaveClass('text-orange-500');
    });

    it('does not highlight a non-bottleneck transition', () => {
      // running: 400ms, done: 600ms out of 1000ms → neither is >50% in first, done is but check running
      const fsm: FiniteStateMachine = {
        ...BASE_FSM,
        transitions: [
          makeTransition('running', 0),
          makeTransition('done', 0.4),
          makeTransition('end', 1),
        ],
      };
      render(<EntityDetailPanel {...DEFAULT_PROPS} fsm={fsm} />);
      const nonBottleneck = screen.getByText('400.00ms');
      expect(nonBottleneck).not.toHaveClass('text-orange-500');
    });
  });
});
