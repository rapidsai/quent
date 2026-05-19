// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from 'vitest';
import type { QueryBundle, EntityRef, Plan, Operator, Port, PlanTree } from '@quent/utils';
import { validateQueryBundle, getTreeData, getPlanDAG } from './query-bundle-transformer';

// ---- Helpers ---------------------------------------------------------------

function makePlan(
  id: string,
  opts: { instanceName?: string | null; edges?: Plan['edges'] } = {}
): Plan {
  return {
    id,
    instance_name: opts.instanceName ?? null,
    parent: null,
    worker_id: null,
    edges: opts.edges ?? [],
  };
}

function makeOperator(
  id: string,
  opts: { typeName?: string | null; instanceName?: string | null } = {}
): Operator {
  return {
    id,
    plan_id: null,
    parent_operator_ids: [],
    instance_name: opts.instanceName ?? null,
    operator_type_name: opts.typeName ?? null,
    custom_attributes: {},
    statistics: null,
    active_span: null,
  };
}

function makePort(id: string, operatorId: string | null): Port {
  return { id, operator_id: operatorId, instance_name: null, statistics: null };
}

function makePlanTree(
  id: string,
  worker: string | null = null,
  children: PlanTree[] = []
): PlanTree {
  return { id, worker, children };
}

function makeBundle(
  plans: Record<string, Plan | undefined>,
  opts: {
    operators?: Record<string, Operator | undefined>;
    ports?: Record<string, Port | undefined>;
    planTree?: PlanTree;
  } = {}
): QueryBundle<EntityRef> {
  return {
    entities: {
      plans,
      operators: opts.operators ?? {},
      ports: opts.ports ?? {},
    },
    plan_tree: opts.planTree ?? makePlanTree(Object.keys(plans)[0] ?? 'p1'),
  } as unknown as QueryBundle<EntityRef>;
}

// ---- validateQueryBundle ---------------------------------------------------

describe('validateQueryBundle', () => {
  it('returns true for a valid bundle with at least one plan', () => {
    const bundle = makeBundle({ p1: makePlan('p1') });
    expect(validateQueryBundle(bundle)).toBe(true);
  });

  it('returns false for null', () => {
    expect(validateQueryBundle(null as unknown as QueryBundle<EntityRef>)).toBe(false);
  });

  it('returns false for a bundle with an empty plans object', () => {
    const bundle = makeBundle({});
    expect(validateQueryBundle(bundle)).toBe(false);
  });

  it('returns true when there are multiple plans', () => {
    const bundle = makeBundle({ p1: makePlan('p1'), p2: makePlan('p2') });
    expect(validateQueryBundle(bundle)).toBe(true);
  });
});

// ---- getTreeData -----------------------------------------------------------

describe('getTreeData', () => {
  it('throws for an invalid bundle', () => {
    expect(() => getTreeData(null as unknown as QueryBundle<EntityRef>)).toThrow(
      'Invalid QueryBundle format'
    );
  });

  it('returns an array of length 1', () => {
    const bundle = makeBundle({ p1: makePlan('p1') }, { planTree: makePlanTree('p1') });
    expect(getTreeData(bundle)).toHaveLength(1);
  });

  it('sets id to the plan_tree id', () => {
    const bundle = makeBundle({ p1: makePlan('p1') }, { planTree: makePlanTree('p1') });
    expect(getTreeData(bundle)[0]!.id).toBe('p1');
  });

  it('sets name to "Query Plan: <id>"', () => {
    const bundle = makeBundle({ p1: makePlan('p1') }, { planTree: makePlanTree('p1') });
    expect(getTreeData(bundle)[0]!.name).toBe('Query Plan: p1');
  });

  it('sets queryId to the plan_tree id', () => {
    const bundle = makeBundle({ p1: makePlan('p1') }, { planTree: makePlanTree('p1') });
    expect(getTreeData(bundle)[0]!.queryId).toBe('p1');
  });

  it('sets workerId from plan_tree.worker', () => {
    const bundle = makeBundle({ p1: makePlan('p1') }, { planTree: makePlanTree('p1', 'worker-7') });
    expect(getTreeData(bundle)[0]!.workerId).toBe('worker-7');
  });

  it('sets workerId to undefined when plan_tree.worker is null', () => {
    const bundle = makeBundle({ p1: makePlan('p1') }, { planTree: makePlanTree('p1', null) });
    expect(getTreeData(bundle)[0]!.workerId).toBeUndefined();
  });

  it('sets planType from the matching plan instance_name', () => {
    const bundle = makeBundle(
      { p1: makePlan('p1', { instanceName: 'HashJoin' }) },
      { planTree: makePlanTree('p1') }
    );
    expect(getTreeData(bundle)[0]!.planType).toBe('HashJoin');
  });

  it('sets planType to undefined when instance_name is null', () => {
    const bundle = makeBundle(
      { p1: makePlan('p1', { instanceName: null }) },
      { planTree: makePlanTree('p1') }
    );
    expect(getTreeData(bundle)[0]!.planType).toBeUndefined();
  });

  it('sets className to "rounded-none"', () => {
    const bundle = makeBundle({ p1: makePlan('p1') }, { planTree: makePlanTree('p1') });
    expect(getTreeData(bundle)[0]!.className).toBe('rounded-none');
  });

  it('sets children to undefined when plan_tree has no children', () => {
    const bundle = makeBundle({ p1: makePlan('p1') }, { planTree: makePlanTree('p1', null, []) });
    expect(getTreeData(bundle)[0]!.children).toBeUndefined();
  });

  it('recursively maps children', () => {
    const childTree = makePlanTree('p2', 'worker-2');
    const rootTree = makePlanTree('p1', null, [childTree]);
    const bundle = makeBundle(
      { p1: makePlan('p1'), p2: makePlan('p2', { instanceName: 'Merge' }) },
      { planTree: rootTree }
    );
    const root = getTreeData(bundle)[0]!;
    expect(root.children).toHaveLength(1);
    const child = root.children![0]!;
    expect(child.id).toBe('p2');
    expect(child.name).toBe('Query Plan: p2');
    expect(child.workerId).toBe('worker-2');
    expect(child.planType).toBe('Merge');
  });
});

// ---- getPlanDAG ------------------------------------------------------------

describe('getPlanDAG', () => {
  it('throws for an invalid bundle', () => {
    expect(() => getPlanDAG(null as unknown as QueryBundle<EntityRef>, 'p1')).toThrow(
      'Invalid QueryBundle format'
    );
  });

  it('throws when all plan values are undefined (no usable plans)', () => {
    const bundle = {
      entities: { plans: { placeholder: undefined }, operators: {}, ports: {} },
      plan_tree: makePlanTree('placeholder'),
    } as unknown as QueryBundle<EntityRef>;
    expect(() => getPlanDAG(bundle, 'missing')).toThrow('No plan found for planId:');
  });

  it('returns empty nodes and edges for a plan with no edges', () => {
    const bundle = makeBundle({ p1: makePlan('p1', { edges: [] }) });
    const result = getPlanDAG(bundle, 'p1');
    expect(result.nodes).toHaveLength(0);
    expect(result.edges).toHaveLength(0);
  });

  it('builds a node for each unique operator referenced by edges', () => {
    const op1 = makeOperator('op1', { typeName: 'Scan' });
    const op2 = makeOperator('op2', { typeName: 'Join' });
    const port1 = makePort('port1', 'op1');
    const port2 = makePort('port2', 'op2');
    const plan = makePlan('p1', { edges: [{ source: 'port1', target: 'port2' }] });
    const bundle = makeBundle({ p1: plan }, { operators: { op1, op2 }, ports: { port1, port2 } });
    const result = getPlanDAG(bundle, 'p1');
    expect(result.nodes).toHaveLength(2);
    const ids = result.nodes.map(n => n.id);
    expect(ids).toContain('op1');
    expect(ids).toContain('op2');
  });

  it('deduplicates nodes when the same operator appears in multiple edges', () => {
    const op1 = makeOperator('op1', { typeName: 'Scan' });
    const op2 = makeOperator('op2', { typeName: 'Join' });
    const op3 = makeOperator('op3', { typeName: 'Agg' });
    const port1 = makePort('port1', 'op1');
    const port2 = makePort('port2', 'op2');
    const port3 = makePort('port3', 'op2'); // op2 reused as source
    const port4 = makePort('port4', 'op3');
    const plan = makePlan('p1', {
      edges: [
        { source: 'port1', target: 'port2' },
        { source: 'port3', target: 'port4' },
      ],
    });
    const bundle = makeBundle(
      { p1: plan },
      { operators: { op1, op2, op3 }, ports: { port1, port2, port3, port4 } }
    );
    const result = getPlanDAG(bundle, 'p1');
    expect(result.nodes).toHaveLength(3); // op2 deduplicated
  });

  it('builds edges with id "<source-port>-<target-port>"', () => {
    const op1 = makeOperator('op1', { typeName: 'Scan' });
    const op2 = makeOperator('op2', { typeName: 'Join' });
    const port1 = makePort('port1', 'op1');
    const port2 = makePort('port2', 'op2');
    const plan = makePlan('p1', { edges: [{ source: 'port1', target: 'port2' }] });
    const bundle = makeBundle({ p1: plan }, { operators: { op1, op2 }, ports: { port1, port2 } });
    const result = getPlanDAG(bundle, 'p1');
    expect(result.edges[0]!.id).toBe('port1-port2');
  });

  it('sets edge type to "smoothstep"', () => {
    const op1 = makeOperator('op1', { typeName: 'Scan' });
    const op2 = makeOperator('op2', { typeName: 'Join' });
    const port1 = makePort('port1', 'op1');
    const port2 = makePort('port2', 'op2');
    const plan = makePlan('p1', { edges: [{ source: 'port1', target: 'port2' }] });
    const bundle = makeBundle({ p1: plan }, { operators: { op1, op2 }, ports: { port1, port2 } });
    const result = getPlanDAG(bundle, 'p1');
    expect(result.edges[0]!.type).toBe('smoothstep');
  });

  it('sets edge source/target to operator IDs (not port IDs)', () => {
    const op1 = makeOperator('op1', { typeName: 'Scan' });
    const op2 = makeOperator('op2', { typeName: 'Join' });
    const port1 = makePort('port1', 'op1');
    const port2 = makePort('port2', 'op2');
    const plan = makePlan('p1', { edges: [{ source: 'port1', target: 'port2' }] });
    const bundle = makeBundle({ p1: plan }, { operators: { op1, op2 }, ports: { port1, port2 } });
    const result = getPlanDAG(bundle, 'p1');
    expect(result.edges[0]!.source).toBe('op1');
    expect(result.edges[0]!.target).toBe('op2');
  });

  it('skips edges where the source port is missing from the bundle', () => {
    const op2 = makeOperator('op2', { typeName: 'Join' });
    const port2 = makePort('port2', 'op2');
    // port1 is intentionally absent
    const plan = makePlan('p1', { edges: [{ source: 'port1', target: 'port2' }] });
    const bundle = makeBundle({ p1: plan }, { operators: { op2 }, ports: { port2 } });
    const result = getPlanDAG(bundle, 'p1');
    expect(result.nodes).toHaveLength(0);
    expect(result.edges).toHaveLength(0);
  });

  it('skips edges where a port has no operator_id', () => {
    const op2 = makeOperator('op2', { typeName: 'Join' });
    const port1 = makePort('port1', null); // no linked operator
    const port2 = makePort('port2', 'op2');
    const plan = makePlan('p1', { edges: [{ source: 'port1', target: 'port2' }] });
    const bundle = makeBundle({ p1: plan }, { operators: { op2 }, ports: { port1, port2 } });
    const result = getPlanDAG(bundle, 'p1');
    expect(result.nodes).toHaveLength(0);
    expect(result.edges).toHaveLength(0);
  });

  it('uses instance_name as node label when set', () => {
    const op1 = makeOperator('op1', { typeName: 'Scan', instanceName: 'ScanAlias' });
    const op2 = makeOperator('op2', { typeName: 'Join' });
    const port1 = makePort('port1', 'op1');
    const port2 = makePort('port2', 'op2');
    const plan = makePlan('p1', { edges: [{ source: 'port1', target: 'port2' }] });
    const bundle = makeBundle({ p1: plan }, { operators: { op1, op2 }, ports: { port1, port2 } });
    const result = getPlanDAG(bundle, 'p1');
    expect(result.nodes.find(n => n.id === 'op1')!.label).toBe('ScanAlias');
  });

  it('falls back to operator_type_name as label when instance_name is null', () => {
    const op1 = makeOperator('op1', { typeName: 'Scan', instanceName: null });
    const op2 = makeOperator('op2', { typeName: 'Join' });
    const port1 = makePort('port1', 'op1');
    const port2 = makePort('port2', 'op2');
    const plan = makePlan('p1', { edges: [{ source: 'port1', target: 'port2' }] });
    const bundle = makeBundle({ p1: plan }, { operators: { op1, op2 }, ports: { port1, port2 } });
    const result = getPlanDAG(bundle, 'p1');
    expect(result.nodes.find(n => n.id === 'op1')!.label).toBe('Scan');
  });

  it('falls back to "Node" as label when both instance_name and operator_type_name are null', () => {
    const op1 = makeOperator('op1', { typeName: null, instanceName: null });
    const op2 = makeOperator('op2', { typeName: 'Join' });
    const port1 = makePort('port1', 'op1');
    const port2 = makePort('port2', 'op2');
    const plan = makePlan('p1', { edges: [{ source: 'port1', target: 'port2' }] });
    const bundle = makeBundle({ p1: plan }, { operators: { op1, op2 }, ports: { port1, port2 } });
    const result = getPlanDAG(bundle, 'p1');
    expect(result.nodes.find(n => n.id === 'op1')!.label).toBe('Node');
  });

  it('sets node type to lowercased operator_type_name', () => {
    const op1 = makeOperator('op1', { typeName: 'HashJoin' });
    const op2 = makeOperator('op2', { typeName: 'Scan' });
    const port1 = makePort('port1', 'op1');
    const port2 = makePort('port2', 'op2');
    const plan = makePlan('p1', { edges: [{ source: 'port1', target: 'port2' }] });
    const bundle = makeBundle({ p1: plan }, { operators: { op1, op2 }, ports: { port1, port2 } });
    const result = getPlanDAG(bundle, 'p1');
    expect(result.nodes.find(n => n.id === 'op1')!.type).toBe('hashjoin');
  });

  it('falls back to "operator" as type when operator_type_name is null', () => {
    const op1 = makeOperator('op1', { typeName: null });
    const op2 = makeOperator('op2', { typeName: 'Join' });
    const port1 = makePort('port1', 'op1');
    const port2 = makePort('port2', 'op2');
    const plan = makePlan('p1', { edges: [{ source: 'port1', target: 'port2' }] });
    const bundle = makeBundle({ p1: plan }, { operators: { op1, op2 }, ports: { port1, port2 } });
    const result = getPlanDAG(bundle, 'p1');
    expect(result.nodes.find(n => n.id === 'op1')!.type).toBe('operator');
  });

  it('falls back to plans[0] when planId does not match any plan', () => {
    const op1 = makeOperator('op1', { typeName: 'Scan' });
    const op2 = makeOperator('op2', { typeName: 'Join' });
    const port1 = makePort('port1', 'op1');
    const port2 = makePort('port2', 'op2');
    const plan = makePlan('p1', { edges: [{ source: 'port1', target: 'port2' }] });
    const bundle = makeBundle({ p1: plan }, { operators: { op1, op2 }, ports: { port1, port2 } });
    // 'no-such-plan' doesn't exist; should fall back to p1
    const result = getPlanDAG(bundle, 'no-such-plan');
    expect(result.nodes).toHaveLength(2);
    expect(result.edges).toHaveLength(1);
  });

  it('attaches the raw operator to node metadata', () => {
    const op1 = makeOperator('op1', { typeName: 'Scan' });
    const op2 = makeOperator('op2', { typeName: 'Join' });
    const port1 = makePort('port1', 'op1');
    const port2 = makePort('port2', 'op2');
    const plan = makePlan('p1', { edges: [{ source: 'port1', target: 'port2' }] });
    const bundle = makeBundle({ p1: plan }, { operators: { op1, op2 }, ports: { port1, port2 } });
    const result = getPlanDAG(bundle, 'p1');
    expect(result.nodes.find(n => n.id === 'op1')!.metadata!.rawNode).toBe(op1);
  });
});
