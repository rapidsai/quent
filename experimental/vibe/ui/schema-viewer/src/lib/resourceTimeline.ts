// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { Schema } from '@quent/schema';

import { buildResources, parseFsm, pathKey } from './schema';
import type {
  EntityGraphModel,
  EntityGraphNode,
  EntityReference,
  FsmTopology,
  ResourceCapacity,
  SchemaPath,
} from './types';

export interface ResourceTimelineStateStep {
  id: string;
  name: string;
  width: number;
  usesResource: boolean;
  capacities: string[];
}

export interface ResourceTimelineFsmSequence {
  id: string;
  entity: SchemaPath;
  label: string;
  states: ResourceTimelineStateStep[];
}

export interface ResourceTimelineCapacityTrace {
  id: string;
  name: string;
  kind: ResourceCapacity['kind'] | 'unspecified';
  bounded: boolean;
  bins: ResourceTimelineCapacityBin[];
}

export interface ResourceTimelineCapacityBin {
  id: string;
  height: number;
}

export interface ResourceTimelineRow {
  node: EntityGraphNode;
  resourceInScope: boolean;
  parentId: string | null;
  depth: number;
  sequences: ResourceTimelineFsmSequence[];
  capacities: ResourceTimelineCapacityTrace[];
}

export interface ResourceTimelineLayout {
  hasResources: boolean;
  filteredCount: number;
  rows: ResourceTimelineRow[];
  rowsById: Map<string, ResourceTimelineRow>;
}

export function buildResourceTimeline(
  model: EntityGraphModel,
  schema: Schema | null,
): ResourceTimelineLayout {
  const nodes = new Map(model.nodes.map((node) => [node.id, node]));
  const parents = treeParents(model.references, nodes);
  const retained = retainedResourceTreeNodes(model.nodes, parents);
  const retainedOrder = orderedTreeNodes(
    model.nodes,
    parents,
    (node) => retained.has(node.id),
  );
  const filteredOrder = orderedTreeNodes(
    model.nodes,
    parents,
    (node) => !retained.has(node.id),
  );

  const resourceData = buildResourceTimelineData(schema);
  const rows: ResourceTimelineRow[] = [];
  const appendRows = (
    ordered: OrderedTreeNode[],
    resourceInScope: boolean,
  ): void => {
    for (const { node, parentId, depth } of ordered) {
      const nodeResourceData = resourceInScope
        ? resourceData.get(node.id)
        : undefined;
      rows.push({
        node,
        resourceInScope,
        parentId,
        depth,
        sequences: nodeResourceData?.sequences ?? [],
        capacities: nodeResourceData?.capacities ?? [],
      });
    }
  };
  appendRows(retainedOrder, true);
  if (filteredOrder.length > 0) {
    appendRows(filteredOrder, false);
  }

  return {
    hasResources: retained.size > 0,
    filteredCount: filteredOrder.length,
    rows,
    rowsById: new Map(rows.map((row) => [row.node.id, row])),
  };
}

interface OrderedTreeNode {
  node: EntityGraphNode;
  parentId: string | null;
  depth: number;
}

function orderedTreeNodes(
  nodes: EntityGraphNode[],
  parents: Map<string, string>,
  included: (node: EntityGraphNode) => boolean,
): OrderedTreeNode[] {
  const includedIds = new Set(
    nodes.filter(included).map((node) => node.id),
  );
  const children = new Map<string, EntityGraphNode[]>();
  const roots: EntityGraphNode[] = [];

  for (const node of nodes) {
    if (!includedIds.has(node.id)) {
      continue;
    }
    const parentId = parents.get(node.id);
    if (!parentId || !includedIds.has(parentId)) {
      roots.push(node);
      continue;
    }
    const siblings = children.get(parentId) ?? [];
    siblings.push(node);
    children.set(parentId, siblings);
  }

  roots.sort(compareNodes);
  for (const siblings of children.values()) {
    siblings.sort(compareNodes);
  }

  const ordered: OrderedTreeNode[] = [];
  const visited = new Set<string>();
  for (const root of roots) {
    appendSubtree(root, null, 0, children, visited, ordered);
  }
  for (const node of [...nodes].sort(compareNodes)) {
    if (includedIds.has(node.id) && !visited.has(node.id)) {
      appendSubtree(node, null, 0, children, visited, ordered);
    }
  }
  return ordered;
}

function retainedResourceTreeNodes(
  nodes: EntityGraphNode[],
  parents: Map<string, string>,
): Set<string> {
  const retained = new Set<string>();
  for (const node of nodes) {
    if (!node.resource) {
      continue;
    }
    let current: string | undefined = node.id;
    const branch = new Set<string>();
    while (current && !branch.has(current)) {
      branch.add(current);
      retained.add(current);
      current = parents.get(current);
    }
  }
  return retained;
}

interface ResourceTimelineData {
  sequences: ResourceTimelineFsmSequence[];
  capacities: ResourceTimelineCapacityTrace[];
}

function buildResourceTimelineData(
  schema: Schema | null,
): Map<string, ResourceTimelineData> {
  if (!schema) {
    return new Map();
  }
  const entities = new Map(
    schema.entities.map(([path, entity]) => [pathKey(path), entity]),
  );
  const output = new Map<string, ResourceTimelineData>();

  for (const resource of buildResources(schema)) {
    const resourceCapacities =
      resource.capacities.length > 0
        ? resource.capacities
        : [
            {
              name: 'usage',
              kind: 'unspecified' as const,
              bounded: false,
            },
          ];
    const consumers = new Map<
      string,
      Map<string, Set<string>>
    >();
    for (const usage of resource.usages) {
      const usageCapacities =
        usage.fields.length > 0
          ? usage.fields
          : resourceCapacities.map((capacity) => capacity.name);
      for (const consumer of usage.consumers) {
        const entityId = pathKey(consumer.entity);
        const states =
          consumers.get(entityId) ?? new Map<string, Set<string>>();
        const capacities =
          states.get(consumer.event) ?? new Set<string>();
        for (const field of usageCapacities) {
          capacities.add(field);
        }
        states.set(consumer.event, capacities);
        consumers.set(entityId, states);
      }
    }

    const sequences = Array.from(consumers, ([entityId, usageStates]) => {
      const entity = entities.get(entityId);
      const topology = entity ? parseFsm(entity) : null;
      if (!entity || !topology) {
        return null;
      }
      const targetStates = Array.from(usageStates).filter(([state]) =>
        topology.states.includes(state),
      );
      if (targetStates.length === 0) {
        return null;
      }
      return createFsmSequence(
        entity.path,
        topology,
        new Map(targetStates),
        `${pathKey(resource.resource)}:${entityId}`,
      );
    }).filter(
      (sequence): sequence is ResourceTimelineFsmSequence =>
        sequence !== null,
    );
    sequences.sort((left, right) => left.label.localeCompare(right.label));
    const capacities = resourceCapacities.map((capacity) => {
      const contributionCount = sequences.reduce(
        (count, sequence) =>
          count +
          sequence.states.filter((state) =>
            state.capacities.includes(capacity.name),
          ).length,
        0,
      );
      return createCapacityTrace(
        capacity,
        `${pathKey(resource.resource)}:${capacity.name}`,
        contributionCount,
      );
    });
    output.set(pathKey(resource.resource), { sequences, capacities });
  }
  return output;
}

function createFsmSequence(
  entity: SchemaPath,
  topology: FsmTopology,
  usageStates: Map<string, Set<string>>,
  id: string,
): ResourceTimelineFsmSequence {
  const stateNames = topology.states;
  const gap = 0.4;
  const width =
    (100 - gap * Math.max(0, stateNames.length - 1)) /
    Math.max(1, stateNames.length);
  return {
    id,
    entity,
    label: entity.name,
    states: stateNames.map((name, index) => {
      return {
        id: `${id}:${index}:${name}`,
        name,
        width,
        usesResource: usageStates.has(name),
        capacities: Array.from(usageStates.get(name) ?? []).sort(),
      };
    }),
  };
}

function createCapacityTrace(
  capacity: Pick<
    ResourceTimelineCapacityTrace,
    'name' | 'kind' | 'bounded'
  >,
  id: string,
  contributionCount: number,
): ResourceTimelineCapacityTrace {
  const pattern = [2, 5, 3, 7, 4, 8, 6, 3] as const;
  const binCount = 24;
  const bins = Array.from({ length: binCount }, (_, index) => {
    const height =
      contributionCount === 0
        ? 1
        : Math.min(
            16,
            2 + contributionCount * 0.75 + pattern[index % pattern.length]!,
          );
    return {
      id: `${id}:${index}`,
      height,
    };
  });
  return {
    ...capacity,
    id,
    bins,
  };
}

function treeParents(
  references: EntityReference[],
  nodes: Map<string, EntityGraphNode>,
): Map<string, string> {
  const parents = new Map<string, string>();
  for (const reference of references) {
    if (!reference.tree || !reference.target) {
      continue;
    }
    const source = pathKey(reference.source);
    const target = pathKey(reference.target);
    if (
      source !== target &&
      nodes.has(source) &&
      nodes.has(target) &&
      !parents.has(source)
    ) {
      parents.set(source, target);
    }
  }
  return parents;
}

function appendSubtree(
  node: EntityGraphNode,
  parentId: string | null,
  depth: number,
  children: Map<string, EntityGraphNode[]>,
  visited: Set<string>,
  output: OrderedTreeNode[],
): void {
  if (visited.has(node.id)) {
    return;
  }
  visited.add(node.id);
  output.push({ node, parentId, depth });
  for (const child of children.get(node.id) ?? []) {
    appendSubtree(child, node.id, depth + 1, children, visited, output);
  }
}

function compareNodes(left: EntityGraphNode, right: EntityGraphNode): number {
  return left.id.localeCompare(right.id);
}
