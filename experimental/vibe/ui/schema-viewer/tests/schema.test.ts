// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, test } from 'vitest';
import type { DataType } from '@quent/schema';

import { sampleSchema } from './fixtures/sample-schema';
import { FSM_CONSTRAINT } from '../src/lib/constants';
import { layoutEntityGraph } from '../src/lib/layout';
import {
  buildEntityGraph,
  buildResources,
  parseFsm,
  pathKey,
  referenceMatchesFilter,
} from '../src/lib/schema';

describe('schema adapters', () => {
  test('stress fixture covers schema and built-in constraint variants', () => {
    const typeKinds = new Set<string>();
    const constraintNames = new Set<string>(
      Object.keys(sampleSchema.annotations.constraints),
    );
    const visitType = (type: DataType): void => {
      if (typeof type === 'string') {
        typeKinds.add(type);
        return;
      }
      if ('Option' in type) {
        typeKinds.add('Option');
        visitType(type.Option);
        return;
      }
      if ('List' in type) {
        typeKinds.add('List');
        visitType(type.List);
        return;
      }
      if ('Record' in type) {
        typeKinds.add('Record');
        return;
      }
      typeKinds.add('EntityRef');
      for (const name of Object.keys(type.EntityRef.annotations.constraints)) {
        constraintNames.add(name);
      }
      if (type.EntityRef.data) {
        visitType(type.EntityRef.data);
      }
    };

    for (const [, value] of [
      ...sampleSchema.entities,
      ...sampleSchema.records,
    ]) {
      for (const name of Object.keys(value.annotations.constraints)) {
        constraintNames.add(name);
      }
      const fieldMaps =
        'events' in value
          ? Object.values(value.events).map((item) => item.payload)
          : [value.fields];
      for (const fields of fieldMaps) {
        for (const item of Object.values(fields)) {
          visitType(item.ty);
        }
      }
    }

    expect(typeKinds).toEqual(
      new Set([
        'Bool',
        'Uuid',
        'String',
        'U8',
        'U16',
        'U32',
        'U64',
        'I8',
        'I16',
        'I32',
        'I64',
        'F32',
        'F64',
        'DynamicRecord',
        'Option',
        'List',
        'Record',
        'EntityRef',
      ]),
    );
    expect([...constraintNames]).toEqual(
      expect.arrayContaining([
        'quent.ref-target.v0.1.0',
        'quent.ref-tree.v0.1.0',
        'quent.fsm.v0.1.0',
        'quent.resource.v0.1.0',
      ]),
    );
    expect(sampleSchema.records).toHaveLength(21);
    expect(
      sampleSchema.entities.filter(([, value]) => parseFsm(value)),
    ).toHaveLength(9);
    expect(sampleSchema.annotations.docs).not.toBeNull();
    expect(Object.keys(sampleSchema.annotations.metadata)).not.toHaveLength(0);
  });

  test('extracts entity references and tree roles', () => {
    const graph = buildEntityGraph(sampleSchema);

    expect(graph.nodes).toHaveLength(26);
    expect(graph.references).toHaveLength(52);
    expect(graph.nodes.filter((node) => node.fsm)).toHaveLength(9);
    expect(graph.nodes.filter((node) => node.resource)).toHaveLength(6);
    expect(graph.references.filter((reference) => reference.tree)).toHaveLength(
      26,
    );
    expect(
      Object.fromEntries(
        (['all', 'untyped', 'typed', 'tree'] as const).map((filter) => [
          filter,
          graph.references.filter((reference) =>
            referenceMatchesFilter(reference, filter)
          ).length,
        ]),
      ),
    ).toEqual({
      all: 52,
      untyped: 1,
      typed: 51,
      tree: 26,
    });
    expect(
      new Set(
        graph.references
          .filter((reference) => reference.tree)
          .map((reference) => pathKey(reference.source)),
      ).size,
    ).toBe(25);
    expect(
      graph.references.some(
        (reference) =>
          pathKey(reference.source) === 'Service::Gateway' &&
          reference.target &&
          pathKey(reference.target) === 'Service::Database',
      ),
    ).toBe(true);
    expect(
      graph.references.some(
        (reference) =>
          pathKey(reference.source) === 'Service::Database' &&
          reference.target &&
          pathKey(reference.target) === 'Service::Gateway',
      ),
    ).toBe(true);
    expect(
      graph.references.some(
        (reference) =>
          pathKey(reference.source) === 'Workload::Dataset' &&
          reference.target &&
          pathKey(reference.target) === 'Workload::Dataset',
      ),
    ).toBe(true);
  });

  test('lays out tree parents above their children', async () => {
    const graph = buildEntityGraph(sampleSchema);
    const layout = await layoutEntityGraph(graph);
    const nodes = new Map(layout.nodes.map((node) => [node.id, node]));
    const nodeIds = new Set(graph.nodes.map((node) => node.id));
    const resolvableReferences = graph.references.filter(
      (reference) =>
        reference.target && nodeIds.has(pathKey(reference.target)),
    );

    expect(layout.nodes).toHaveLength(26);
    expect(layout.references).toHaveLength(resolvableReferences.length);
    expect(
      layout.references.every((reference) =>
        reference.sections.every(
          (section) =>
            Number.isFinite(section.startPoint.x) &&
            Number.isFinite(section.startPoint.y) &&
            Number.isFinite(section.endPoint.x) &&
            Number.isFinite(section.endPoint.y),
        ),
      ),
    ).toBe(true);
    for (const reference of layout.references) {
      const source = nodes.get(reference.source)!;
      const target = nodes.get(reference.target)!;
      const endpoints = reference.sections.flatMap((section) => [
        section.startPoint,
        section.endPoint,
      ]);
      expect(
        Math.min(
          ...endpoints.map((point) => distanceToNode(point, source)),
        ),
        `${reference.id} source`,
      ).toBeLessThan(2);
      expect(
        Math.min(
          ...endpoints.map((point) => distanceToNode(point, target)),
        ),
        `${reference.id} target`,
      ).toBeLessThan(2);
    }
    expect(layout.width).toBeLessThan(2_600);
    expect(layout.height).toBeLessThan(2_000);
    expect(layout.groups.map((group) => group.label)).toEqual(
      expect.arrayContaining([
        'Infrastructure',
        'Workload',
        'Service',
        'Observability',
        'Security',
      ]),
    );
    const serviceGroup = layout.groups.find(
      (group) => group.label === 'Service',
    )!;
    const workerNode = nodes.get('Service::Worker')!;
    expect(workerNode.x).toBeGreaterThan(serviceGroup.x);
    expect(workerNode.y).toBeGreaterThan(serviceGroup.y);
    expect(workerNode.x + workerNode.width).toBeLessThan(
      serviceGroup.x + serviceGroup.width,
    );
    expect(workerNode.y + workerNode.height).toBeLessThan(
      serviceGroup.y + serviceGroup.height,
    );
    expect(nodes.get('Platform')!.y).toBeLessThan(
      nodes.get('Infrastructure::Region')!.y,
    );
    expect(nodes.get('Infrastructure::Region')!.y).toBeLessThan(
      nodes.get('Infrastructure::Cluster')!.y,
    );
    expect(nodes.get('Infrastructure::Cluster')!.y).toBeLessThan(
      nodes.get('Infrastructure::Node')!.y,
    );

    const gatewayCycle = layout.references.filter((value) => {
      const source = pathKey(value.reference.source);
      const target = value.reference.target
        ? pathKey(value.reference.target)
        : '';
      return (
        (source === 'Service::Gateway' &&
          target === 'Service::Database') ||
        (source === 'Service::Database' &&
          target === 'Service::Gateway')
      );
    });
    expect(gatewayCycle).toHaveLength(2);

    const datasetParents = layout.references.filter(
      (value) =>
        value.reference.tree &&
        pathKey(value.reference.source) === 'Workload::Dataset',
    );
    expect(datasetParents).toHaveLength(2);
    expect(
      layout.nodes.find((node) => node.id === 'Service::Worker')?.parentId,
    ).toBe('namespace:Service');
  });

  test.each([
    ['layering', { layeringStrategy: 'coffman-graham' } as const],
    ['placement', { nodePlacementStrategy: 'network-simplex' } as const],
    ['hierarchical crossings', { hierarchicalGreedySwitch: true } as const],
    ['effort', { layoutThoroughness: 20 } as const],
    ['high degree', { highDegreeNodeTreatment: true } as const],
  ])('supports configurable layered %s strategy', async (_name, config) => {
    const graph = buildEntityGraph(sampleSchema);
    const layout = await layoutEntityGraph(graph, config);

    expect(layout.nodes).toHaveLength(graph.nodes.length);
    expect(layout.references.length).toBeGreaterThan(0);
    expect(layout.nodes.every(({ x, y }) =>
      Number.isFinite(x) && Number.isFinite(y)
    )).toBe(true);
  });

  test('decodes FSM topology', () => {
    const worker = sampleSchema.entities.find(
      ([path]) => pathKey(path) === 'Service::Worker',
    )![1];

    expect(parseFsm(worker)).toEqual({
      initialState: 'starting',
      transitions: [
        { source: 'starting', target: 'idle' },
        { source: 'idle', target: 'busy' },
        { source: 'busy', target: 'idle' },
        { source: 'idle', target: 'draining' },
        { source: 'draining', target: 'stopped' },
      ],
      exitStates: ['stopped'],
      states: ['starting', 'idle', 'busy', 'draining', 'stopped'],
    });
    expect(worker.events.idle.cardinality).toBe('Multi');
    expect(worker.events.busy.cardinality).toBe('Multi');
    expect(worker.events.starting.cardinality).toBe('Once');
  });

  test('adapts an explicit exit state to the topology exit marker', () => {
    const worker = structuredClone(sampleSchema.entities.find(
      ([path]) => pathKey(path) === 'Service::Worker',
    )![1]);
    const exit = structuredClone(worker.events.stopped!);
    delete worker.events.stopped;
    worker.events.exit = {
      ...exit,
      name: 'exit',
    };
    worker.annotations.constraints[FSM_CONSTRAINT]!.data = JSON.stringify({
      initial_state: 'starting',
      transitions: [
        { source: 'starting', target: 'idle' },
        { source: 'idle', target: 'busy' },
        { source: 'busy', target: 'idle' },
        { source: 'idle', target: 'draining' },
        { source: 'draining', target: 'exit' },
      ],
    });

    expect(parseFsm(worker)).toEqual({
      initialState: 'starting',
      transitions: [
        { source: 'starting', target: 'idle' },
        { source: 'idle', target: 'busy' },
        { source: 'busy', target: 'idle' },
        { source: 'idle', target: 'draining' },
      ],
      exitStates: ['draining'],
      states: ['starting', 'idle', 'busy', 'draining'],
    });
  });

  test('joins resource definitions, role records, and consumers', () => {
    const resources = buildResources(sampleSchema);
    expect(resources).toHaveLength(6);

    const memory = resources.find(
      (resource) => pathKey(resource.resource) === 'Resource::Memory',
    )!;
    expect(memory.capacities).toEqual([
      { name: 'bytes', kind: 'occupancy', bounded: true },
      { name: 'bandwidth', kind: 'rate', bounded: true },
    ]);
    expect(
      memory.usages[0].consumers.map((consumer) => pathKey(consumer.entity)),
    ).toEqual([
      'Workload::Task',
      'Service::Worker',
      'Service::Database',
      'Service::Cache',
    ]);

    const slot = resources.find(
      (resource) => pathKey(resource.resource) === 'Resource::ExecutionSlot',
    )!;
    expect(slot.capacities).toEqual([]);
    expect(slot.bounds).toEqual([]);
    expect(slot.usages[0].fields).toEqual([]);
  });
});

function distanceToNode(
  point: { x: number; y: number },
  node: { x: number; y: number; width: number; height: number },
): number {
  const right = node.x + node.width;
  const bottom = node.y + node.height;
  const outsideX = Math.max(node.x - point.x, 0, point.x - right);
  const outsideY = Math.max(node.y - point.y, 0, point.y - bottom);
  if (outsideX > 0 || outsideY > 0) {
    return Math.hypot(outsideX, outsideY);
  }
  return Math.min(
    point.x - node.x,
    right - point.x,
    point.y - node.y,
    bottom - point.y,
  );
}
