// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, test } from 'vitest';

import { sampleSchema } from './fixtures/sample-schema';
import { resolveEntityGraphConfig } from '../src/lib/config';
import {
  layoutEntityGraph,
  layoutFsmTopology,
} from '../src/lib/layout';
import {
  buildEntityGraph,
  parseFsm,
  pathKey,
} from '../src/lib/schema';
import {
  edgeGeometryFromSections,
  READ_ONLY_FLOW_CONFIG,
  toEntityFlowElements,
  toFsmFlowElements,
} from '../src/lib/xyflow';

describe('XYFlow adapter', () => {
  test('uses orthogonal routing and compact layered defaults', () => {
    expect(resolveEntityGraphConfig(undefined)).toMatchObject({
      edgeRouting: 'orthogonal',
      layeringStrategy: 'network-simplex',
      nodePlacementStrategy: 'linear-segments',
    });
  });

  test('resolves bounded ELK layout tuning', () => {
    expect(
      resolveEntityGraphConfig({
        layeringStrategy: 'longest-path',
        nodePlacementStrategy: 'linear-segments',
        hierarchicalGreedySwitch: true,
        layoutThoroughness: 500,
        highDegreeNodeTreatment: true,
      }),
    ).toMatchObject({
      layeringStrategy: 'longest-path',
      nodePlacementStrategy: 'linear-segments',
      hierarchicalGreedySwitch: true,
      layoutThoroughness: 100,
      highDegreeNodeTreatment: true,
    });
  });

  test('converts schema nodes, namespace parents, and cyclic references', async () => {
    const model = buildEntityGraph(sampleSchema);
    const config = resolveEntityGraphConfig({
      groupNamespaces: true,
      references: 'all',
    });
    const layout = await layoutEntityGraph(model, config);
    const flow = toEntityFlowElements({
      schema: sampleSchema,
      layout,
      config,
      classes: {},
      selection: null,
      nodeComponent: null,
    });
    const groups = flow.nodes.filter(
      (node) => node.type === 'quent-namespace',
    );
    const entities = flow.nodes.filter(
      (node) => node.type === 'quent-entity',
    );

    expect(entities).toHaveLength(26);
    expect(groups.length).toBeGreaterThan(0);
    expect(flow.nodes.indexOf(groups[0]!)).toBeLessThan(
      flow.nodes.indexOf(entities[0]!),
    );
    expect(
      entities.find((node) => node.id === 'Service::Worker')?.parentId,
    ).toBe('namespace:Service');
    expect(
      groups.find((node) => node.id === 'namespace:Service'),
    ).toMatchObject({
      selectable: false,
      draggable: false,
      connectable: false,
    });

    const worker = entities.find(
      (node) => node.id === 'Service::Worker',
    )!;
    const memory = entities.find(
      (node) => node.id === 'Resource::Memory',
    )!;
    const plainEntity = entities.find(
      (node) => node.id === 'Platform',
    )!;
    expect(worker.data.node.fsm).toBe(true);
    expect(worker.data.node.resource).toBe(false);
    expect(memory.data.node.resource).toBe(true);
    expect(plainEntity.data.node).toMatchObject({
      fsm: false,
      resource: false,
    });
    expect(
      entities.every(
        (node) =>
          node.draggable === false &&
          node.connectable === false &&
          node.deletable === false,
      ),
    ).toBe(true);

    const cycle = flow.edges.filter((edge) => {
      const reference = edge.data!.reference;
      const source = pathKey(reference.source);
      const target = reference.target ? pathKey(reference.target) : '';
      return (
        (source === 'Service::Gateway' &&
          target === 'Service::Database') ||
        (source === 'Service::Database' &&
          target === 'Service::Gateway')
      );
    });
    expect(cycle).toHaveLength(2);
    expect(cycle.every((edge) => edge.deletable === false)).toBe(true);
    expect(
      flow.edges.every((edge) => {
        const reference = edge.data!.reference;
        return (
          edge.source === pathKey(reference.source) &&
          edge.target === (
            reference.target ? pathKey(reference.target) : ''
          )
        );
      }),
    ).toBe(true);
    expect(flow.edges.every((edge) => edge.type === 'quent-elk')).toBe(true);
    expect(
      flow.edges.every((edge) => edge.data?.path?.startsWith('M ')),
    ).toBe(true);
    expect(
      flow.edges.every(
        (edge) =>
          typeof edge.markerEnd === 'object' &&
          edge.markerEnd.width === 24 &&
          edge.markerEnd.height === 24 &&
          edge.markerEnd.markerUnits === 'userSpaceOnUse',
      ),
    ).toBe(true);
    expect(
      flow.edges
        .filter((edge) => !edge.data?.reference.tree)
        .every(
          (edge) =>
            typeof edge.markerEnd === 'object' &&
            edge.markerEnd.color === 'var(--quent-viewer-muted)',
        ),
    ).toBe(true);
  });

  test('applies reference filtering, labels, and controlled selection', async () => {
    const model = buildEntityGraph(sampleSchema);
    const config = resolveEntityGraphConfig({
      references: 'tree',
      referenceLabels: 'never',
    });
    const layout = await layoutEntityGraph(model, config);
    const selectedEntity = model.nodes.find(
      (node) => node.id === 'Service::Worker',
    )!;
    const flow = toEntityFlowElements({
      schema: sampleSchema,
      layout,
      config,
      classes: {},
      selection: {
        kind: 'entity',
        entity: selectedEntity.path,
      },
      nodeComponent: null,
    });

    expect(flow.edges).toHaveLength(
      model.references.filter(
        (reference) => reference.tree && reference.target,
      ).length,
    );
    expect(flow.edges.every((edge) => edge.label === undefined)).toBe(true);
    expect(
      layout.references.every(
        (reference) => reference.labelPosition === null,
      ),
    ).toBe(true);
    expect(
      flow.edges.every(
        (edge) =>
          edge.class?.includes('quent-entity-graph__edge--tree') &&
          edge.style ===
            '--xy-edge-stroke:var(--quent-viewer-tree);--xy-edge-stroke-width:4' &&
          typeof edge.markerEnd === 'object' &&
          edge.markerEnd.color === 'var(--quent-viewer-tree)',
      ),
    ).toBe(true);
    expect(
      flow.nodes.find((node) => node.id === selectedEntity.id)?.selected,
    ).toBe(true);
  });

  test('uses ELK positions only for always-visible reference labels', async () => {
    const model = buildEntityGraph(sampleSchema);
    const config = resolveEntityGraphConfig({
      references: 'tree',
      referenceLabels: 'always',
    });
    const layout = await layoutEntityGraph(model, config);
    const flow = toEntityFlowElements({
      schema: sampleSchema,
      layout,
      config,
      classes: {},
      selection: null,
      nodeComponent: null,
    });

    expect(
      layout.references.every(
        (reference) => reference.labelPosition !== null,
      ),
    ).toBe(true);
    expect(
      flow.edges.every((edge, index) => {
        const position = layout.references[index]?.labelPosition;
        return (
          edge.data?.labelX === position?.x &&
          edge.data?.labelY === position?.y
        );
      }),
    ).toBe(true);
  });

  test('does not reserve layout space for interaction labels', async () => {
    const model = buildEntityGraph(sampleSchema);
    const config = resolveEntityGraphConfig({
      references: 'tree',
      referenceLabels: 'interaction',
    });
    const layout = await layoutEntityGraph(model, config);
    const flow = toEntityFlowElements({
      schema: sampleSchema,
      layout,
      config,
      classes: {},
      selection: null,
      nodeComponent: null,
    });

    expect(
      layout.references.every(
        (reference) => reference.labelPosition === null,
      ),
    ).toBe(true);
    expect(flow.edges.every((edge) => edge.label !== undefined)).toBe(true);
    expect(
      flow.edges.every(
        (edge) =>
          Number.isFinite(edge.data?.labelX) &&
          Number.isFinite(edge.data?.labelY),
      ),
    ).toBe(true);
    expect(
      flow.edges.every((edge) =>
        edge.class?.includes(
          'quent-entity-graph__edge--interaction-label',
        )),
    ).toBe(true);
  });

  test('exports the read-only interaction contract', () => {
    expect(READ_ONLY_FLOW_CONFIG).toEqual({
      nodesDraggable: false,
      nodesConnectable: false,
      elementsSelectable: true,
      selectNodesOnDrag: false,
      selectionOnDrag: false,
      deleteKey: null,
      panOnDrag: true,
      zoomOnScroll: true,
      zoomOnPinch: true,
      zoomOnDoubleClick: false,
    });
  });

  test('rounds ELK edge bends and labels the longest segment', () => {
    expect(
      edgeGeometryFromSections([
        {
          startPoint: { x: 10, y: 20 },
          bendPoints: [
            { x: 10, y: 50 },
            { x: 70, y: 50 },
          ],
          endPoint: { x: 70, y: 80 },
        },
      ]),
    ).toEqual({
      path:
        'M 10 20 L 10 38 Q 10 50 22 50 ' +
        'L 58 50 Q 70 50 70 62 L 70 80',
      label: { x: 40, y: 50 },
    });
  });

  test('converts an ELK FSM layout into read-only XYFlow elements', async () => {
    const entity = sampleSchema.entities.find(
      ([path]) => pathKey(path) === 'Service::Worker',
    )![1];
    const topology = parseFsm(entity)!;
    const layout = await layoutFsmTopology(topology);
    const initial = layout.states.find(
      (state) => state.state === topology.initialState,
    )!;
    const entry = layout.states.find((state) => state.entry)!;
    const exit = layout.states.find((state) => state.exit)!;
    const regularStates = layout.states.filter(
      (state) => !state.entry && !state.exit,
    );
    const flow = toFsmFlowElements({
      path: entity.path,
      topology,
      layout,
      selection: {
        kind: 'fsm-state',
        entity: entity.path,
        state: 'busy',
      },
      classes: {},
    });

    expect(
      flow.nodes.find((node) => node.data.state === 'busy')?.selected,
    ).toBe(true);
    expect(flow.nodes.every((node) => node.type === 'quent-fsm')).toBe(true);
    expect(entry.y).toBeLessThan(initial.y);
    expect(initial.y).toBe(
      Math.min(...regularStates.map((state) => state.y)),
    );
    expect(exit.y).toBeGreaterThan(
      Math.max(...regularStates.map((state) => state.y)),
    );
    expect(
      flow.nodes.filter((node) => node.data.entry || node.data.exit),
    ).toHaveLength(2);
    expect(
      flow.nodes
        .filter((node) => node.data.entry || node.data.exit)
        .every((node) => node.data.label === ''),
    ).toBe(true);
    expect(flow.edges.every((edge) => edge.type === 'quent-elk')).toBe(true);
    expect(
      flow.edges.every((edge) => edge.data?.path?.startsWith('M ')),
    ).toBe(true);
    expect(flow.edges[0]?.markerEnd).toMatchObject({
      width: 24,
      height: 24,
      markerUnits: 'userSpaceOnUse',
    });
  });
});
