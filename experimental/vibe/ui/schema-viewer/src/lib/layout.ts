// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type {
  ELK,
  ElkExtendedEdge,
  ElkNode,
} from 'elkjs/lib/elk.bundled.js';
import ElkWorker from 'elkjs/lib/elk-worker.min.js?worker';

import { resolveEntityGraphConfig } from './config';
import {
  pathKey,
  referenceLabel,
  referenceMatchesFilter,
} from './schema';
import type {
  EntityGraphConfig,
  EntityGraphModel,
  EntityGraphNode,
  EntityReference,
  FsmTopology,
  ResolvedEntityGraphConfig,
} from './types';

let engine: Promise<ELK> | undefined;

export interface PositionedEntityNode extends EntityGraphNode {
  x: number;
  y: number;
  width: number;
  height: number;
  parentId: string | null;
}

export interface PositionedEntityReference {
  id: string;
  reference: EntityReference;
  source: string;
  target: string;
  sections: PositionedEdgeSection[];
  labelPosition: PositionedEdgePoint | null;
}

export interface PositionedEdgePoint {
  x: number;
  y: number;
}

export interface PositionedEdgeSection {
  startPoint: PositionedEdgePoint;
  bendPoints: PositionedEdgePoint[];
  endPoint: PositionedEdgePoint;
}

export interface PositionedNamespaceGroup {
  id: string;
  label: string;
  path: string[];
  x: number;
  y: number;
  width: number;
  height: number;
  depth: number;
  parentId: string | null;
}

export interface EntityGraphLayout {
  width: number;
  height: number;
  groups: PositionedNamespaceGroup[];
  nodes: PositionedEntityNode[];
  references: PositionedEntityReference[];
}

export interface PositionedFsmState {
  id: string;
  state: string;
  x: number;
  y: number;
  width: number;
  height: number;
  entry: boolean;
  exit: boolean;
}

export interface PositionedFsmTransition {
  id: string;
  source: string;
  target: string;
  sections: PositionedEdgeSection[];
}

export interface FsmGraphLayout {
  width: number;
  height: number;
  states: PositionedFsmState[];
  transitions: PositionedFsmTransition[];
}

interface LayoutEdge {
  edge: ElkExtendedEdge;
  reference: EntityReference;
}

interface NamespaceTree {
  path: string[];
  nodes: EntityGraphNode[];
  children: Map<string, NamespaceTree>;
}

interface DensitySpacing {
  node: number;
  layer: number;
  component: number;
  namespaceNode: number;
  namespaceLayer: number;
}

export function createEmptyEntityGraphLayout(): EntityGraphLayout {
  return {
    width: 0,
    height: 0,
    groups: [],
    nodes: [],
    references: [],
  };
}

export async function layoutEntityGraph(
  model: EntityGraphModel,
  config?: EntityGraphConfig,
): Promise<EntityGraphLayout> {
  if (model.nodes.length === 0) {
    return createEmptyEntityGraphLayout();
  }

  const resolved = resolveEntityGraphConfig(config);
  const spacing = densitySpacing(resolved.density);
  const modelNodes = new Map(model.nodes.map((node) => [node.id, node]));
  const layoutEdges = createLayoutEdges(
    model,
    modelNodes,
    resolved.references,
    resolved.referenceLabels,
  );
  const graph: ElkNode = {
    id: 'quent-entity-graph',
    layoutOptions: {
      'elk.algorithm': 'layered',
      'elk.direction': resolved.direction.toUpperCase(),
      'elk.edgeRouting': resolved.edgeRouting.toUpperCase(),
      'elk.hierarchyHandling': 'INCLUDE_CHILDREN',
      'elk.padding': '[top=16,left=16,bottom=16,right=16]',
      'elk.spacing.nodeNode': String(spacing.node),
      'elk.spacing.edgeNode': '18',
      'elk.spacing.edgeEdge': '6',
      'elk.spacing.edgeLabel': '8',
      'elk.spacing.labelNode': '12',
      'elk.edgeLabels.inline': 'true',
      'elk.layered.spacing.nodeNodeBetweenLayers': String(spacing.layer),
      'elk.layered.spacing.edgeNodeBetweenLayers': '18',
      'elk.layered.spacing.edgeEdgeBetweenLayers': '8',
      'elk.layered.mergeEdges': 'false',
      'elk.layered.feedbackEdges': 'true',
      'elk.layered.edgeLabels.centerLabelPlacementStrategy':
        'SPACE_EFFICIENT_LAYER',
      ...layeredStrategyOptions(resolved),
      'elk.layered.nodePlacement.favorStraightEdges': 'true',
      'elk.separateConnectedComponents': 'true',
      'elk.spacing.componentComponent': String(spacing.component),
    },
    children: namespaceChildren(
      model.nodes,
      resolved,
      spacing,
    ),
    edges: layoutEdges.map(({ edge }) => edge),
  };
  const result = await layoutEngine().then((elk) => elk.layout(graph));
  const flattened = flattenEntityLayout(
    result.children ?? [],
    modelNodes,
    resolved.nodeWidth,
    resolved.nodeHeight,
  );
  const resultEdges = new Map(
    (result.edges ?? []).map((edge) => [edge.id, edge]),
  );
  const groups = new Map(
    flattened.groups.map((group) => [group.id, group]),
  );

  return {
    width: result.width ?? 0,
    height: result.height ?? 0,
    groups: flattened.groups,
    nodes: flattened.nodes,
    references: layoutEdges.flatMap(({ edge, reference }) => {
      const resultEdge = resultEdges.get(edge.id);
      if (!resultEdge || !reference.target) {
        return [];
      }
      const source = pathKey(reference.source);
      const target = pathKey(reference.target);
      const sections = positionEdgeSections(resultEdge, groups);
      return [{
        id: edge.id,
        reference,
        source,
        target,
        sections: edge.sources[0] === source
          ? sections
          : reverseEdgeSections(sections),
        labelPosition: positionEdgeLabel(resultEdge, groups),
      }];
    }),
  };
}

function positionEdgeLabel(
  edge: ElkExtendedEdge,
  groups: Map<string, PositionedNamespaceGroup>,
): PositionedEdgePoint | null {
  const label = edge.labels?.[0];
  if (!label || label.x === undefined || label.y === undefined) {
    return null;
  }
  const container = edge.container
    ? groups.get(edge.container)
    : undefined;
  return {
    x: label.x + (label.width ?? 0) / 2 + (container?.x ?? 0),
    y: label.y + (label.height ?? 0) / 2 + (container?.y ?? 0),
  };
}

function positionEdgeSections(
  edge: ElkExtendedEdge,
  groups: Map<string, PositionedNamespaceGroup>,
): PositionedEdgeSection[] {
  const container = edge.container
    ? groups.get(edge.container)
    : undefined;
  const offsetX = container?.x ?? 0;
  const offsetY = container?.y ?? 0;
  const position = (point: PositionedEdgePoint): PositionedEdgePoint => ({
    x: point.x + offsetX,
    y: point.y + offsetY,
  });
  return (edge.sections ?? []).map((section) => ({
    startPoint: position(section.startPoint),
    bendPoints: (section.bendPoints ?? []).map(position),
    endPoint: position(section.endPoint),
  }));
}

function reverseEdgeSections(
  sections: PositionedEdgeSection[],
): PositionedEdgeSection[] {
  return [...sections].reverse().map((section) => ({
    startPoint: section.endPoint,
    bendPoints: [...section.bendPoints].reverse(),
    endPoint: section.startPoint,
  }));
}

function namespaceChildren(
  nodes: EntityGraphNode[],
  config: ResolvedEntityGraphConfig,
  spacing: DensitySpacing,
): ElkNode[] {
  if (!config.groupNamespaces) {
    return nodes.map((node) => ({
      id: node.id,
      width: config.nodeWidth,
      height: config.nodeHeight,
    }));
  }

  const root: NamespaceTree = {
    path: [],
    nodes: [],
    children: new Map(),
  };
  for (const node of nodes) {
    let group = root;
    for (const segment of node.path.namespace) {
      let child = group.children.get(segment);
      if (!child) {
        child = {
          path: [...group.path, segment],
          nodes: [],
          children: new Map(),
        };
        group.children.set(segment, child);
      }
      group = child;
    }
    group.nodes.push(node);
  }
  return namespaceTreeChildren(
    root,
    config,
    spacing,
  );
}

function namespaceTreeChildren(
  tree: NamespaceTree,
  config: ResolvedEntityGraphConfig,
  spacing: DensitySpacing,
): ElkNode[] {
  return [
    ...tree.nodes.map((node) => ({
      id: node.id,
      width: config.nodeWidth,
      height: config.nodeHeight,
    })),
    ...Array.from(tree.children.values(), (child) => ({
      id: namespaceId(child.path),
      layoutOptions: {
        'elk.algorithm': 'layered',
        'elk.direction': config.direction.toUpperCase(),
        'elk.edgeRouting': config.edgeRouting.toUpperCase(),
        ...layeredStrategyOptions(config),
        'elk.padding': '[top=36,left=12,bottom=12,right=12]',
        'elk.spacing.nodeNode': String(spacing.namespaceNode),
        'elk.layered.spacing.nodeNodeBetweenLayers': String(
          spacing.namespaceLayer,
        ),
      },
      children: namespaceTreeChildren(
        child,
        config,
        spacing,
      ),
    })),
  ];
}

function flattenEntityLayout(
  children: ElkNode[],
  modelNodes: Map<string, EntityGraphNode>,
  nodeWidth: number,
  nodeHeight: number,
  offsetX = 0,
  offsetY = 0,
  depth = 0,
  parentId: string | null = null,
): {
  groups: PositionedNamespaceGroup[];
  nodes: PositionedEntityNode[];
} {
  const groups: PositionedNamespaceGroup[] = [];
  const nodes: PositionedEntityNode[] = [];
  for (const child of children) {
    const x = offsetX + (child.x ?? 0);
    const y = offsetY + (child.y ?? 0);
    const modelNode = modelNodes.get(child.id);
    if (modelNode) {
      nodes.push({
        ...modelNode,
        x,
        y,
        width: child.width ?? nodeWidth,
        height: child.height ?? nodeHeight,
        parentId,
      });
      continue;
    }
    if (!child.id.startsWith('namespace:')) {
      continue;
    }

    const path = child.id.slice('namespace:'.length).split('::');
    groups.push({
      id: child.id,
      label: path.at(-1) ?? '',
      path,
      x,
      y,
      width: child.width ?? 0,
      height: child.height ?? 0,
      depth,
      parentId,
    });
    const nested = flattenEntityLayout(
      child.children ?? [],
      modelNodes,
      nodeWidth,
      nodeHeight,
      x,
      y,
      depth + 1,
      child.id,
    );
    groups.push(...nested.groups);
    nodes.push(...nested.nodes);
  }
  return { groups, nodes };
}

function namespaceId(path: string[]): string {
  return `namespace:${path.join('::')}`;
}

function createLayoutEdges(
  model: EntityGraphModel,
  nodes: Map<string, EntityGraphNode>,
  references: NonNullable<EntityGraphConfig['references']>,
  referenceLabels: 'always' | 'interaction' | 'never',
): LayoutEdge[] {
  return model.references.flatMap((reference, index) => {
    if (
      !reference.target ||
      !referenceMatchesFilter(reference, references)
    ) {
      return [];
    }
    const source = pathKey(reference.source);
    const target = pathKey(reference.target);
    if (!nodes.has(source) || !nodes.has(target)) {
      return [];
    }

    // Parent-first edges guide ELK's layering; rendered paths are restored to
    // the source-to-target direction declared by the schema after layout.
    const reverseTreeReference = reference.tree && source !== target;
    const label = referenceLabel(reference);
    return [{
      reference,
      edge: {
        id: `reference-${index}`,
        sources: [reverseTreeReference ? target : source],
        targets: [reverseTreeReference ? source : target],
        labels: referenceLabels === 'always'
          ? [{
              id: `reference-${index}-label`,
              text: label,
              width: Math.max(36, label.length * 6.5 + 8),
              height: 18,
              layoutOptions: {
                'elk.edgeLabels.placement': 'CENTER',
                'elk.edgeLabels.inline': 'true',
              },
            }]
          : undefined,
        layoutOptions: {
          'elk.layered.priority.direction': reference.tree ? '100' : '1',
          'elk.layered.priority.straightness': reference.tree ? '10' : '1',
        },
      },
    }];
  });
}

function densitySpacing(
  density: 'compact' | 'comfortable' | 'spacious',
): DensitySpacing {
  switch (density) {
    case 'comfortable':
      return {
        node: 28,
        layer: 64,
        component: 36,
        namespaceNode: 22,
        namespaceLayer: 56,
      };
    case 'spacious':
      return {
        node: 44,
        layer: 84,
        component: 52,
        namespaceNode: 32,
        namespaceLayer: 72,
      };
    default:
      return {
        node: 14,
        layer: 48,
        component: 20,
        namespaceNode: 10,
        namespaceLayer: 42,
      };
  }
}

function layeredStrategyOptions(
  config: ResolvedEntityGraphConfig,
): Record<string, string> {
  return {
    'elk.layered.layering.strategy': elkEnum(config.layeringStrategy),
    'elk.layered.nodePlacement.strategy': elkEnum(
      config.nodePlacementStrategy,
    ),
    'elk.layered.crossingMinimization.greedySwitchHierarchical.type':
      config.hierarchicalGreedySwitch ? 'TWO_SIDED' : 'OFF',
    'elk.layered.thoroughness': String(config.layoutThoroughness),
    'elk.layered.highDegreeNodes.treatment': String(
      config.highDegreeNodeTreatment,
    ),
  };
}

function elkEnum(value: string): string {
  return value.replaceAll('-', '_').toUpperCase();
}

function layoutEngine(): Promise<ELK> {
  engine ??= typeof Worker === 'undefined'
    ? import('elkjs/lib/elk.bundled.js').then(
        ({ default: ElkConstructor }) =>
          new ElkConstructor({
            algorithms: ['layered'],
          }),
      )
    : import('elkjs/lib/elk-api.js').then(
        ({ default: ElkConstructor }) =>
          new ElkConstructor({
            algorithms: ['layered'],
            workerFactory: () => new ElkWorker(),
          }) as ELK,
      );
  return engine;
}

export async function layoutFsmTopology(
  topology: FsmTopology,
): Promise<FsmGraphLayout> {
  const stateIds = new Map(
    topology.states.map((state, index) => [state, `fsm-state-${index}`]),
  );
  const entryId = 'fsm-entry';
  const exitId = 'fsm-exit';
  const initialId = stateIds.get(topology.initialState);
  const edges: ElkExtendedEdge[] = initialId
    ? [{
        id: 'fsm-entry-transition',
        sources: [entryId],
        targets: [initialId],
      }]
    : [];
  edges.push(...topology.transitions.flatMap(
    (transition, index) => {
      const source = stateIds.get(transition.source);
      const target = stateIds.get(transition.target);
      return source && target
        ? [{
            id: `fsm-transition-${index}`,
            sources: [source],
            targets: [target],
          }]
        : [];
    },
  ));
  for (const [index, state] of topology.exitStates.entries()) {
    const source = stateIds.get(state);
    if (source) {
      edges.push({
        id: `fsm-exit-transition-${index}`,
        sources: [source],
        targets: [exitId],
      });
    }
  }

  const graph: ElkNode = {
    id: 'quent-fsm-graph',
    layoutOptions: {
      'elk.algorithm': 'layered',
      'elk.direction': 'DOWN',
      'elk.edgeRouting': 'POLYLINE',
      'elk.padding': '[top=20,left=20,bottom=20,right=20]',
      'elk.spacing.nodeNode': '32',
      'elk.spacing.edgeNode': '14',
      'elk.layered.spacing.nodeNodeBetweenLayers': '48',
      'elk.layered.feedbackEdges': 'true',
    },
    children: [
      {
        id: entryId,
        width: 18,
        height: 18,
        layoutOptions: {
          'elk.layered.layering.layerConstraint': 'FIRST_SEPARATE',
        },
      },
      ...topology.states.map((state) => ({
        id: stateIds.get(state)!,
        width: 160,
        height: 44,
        layoutOptions: state === topology.initialState
          ? {
              'elk.layered.layering.layerConstraint': 'FIRST',
            }
          : undefined,
      })),
      {
        id: exitId,
        width: 24,
        height: 24,
        layoutOptions: {
          'elk.layered.layering.layerConstraint': 'LAST_SEPARATE',
        },
      },
    ],
    edges,
  };
  const result = await layoutEngine().then((elk) => elk.layout(graph));
  const stateById = new Map(
    Array.from(stateIds, ([state, id]) => [id, state]),
  );
  const states = (result.children ?? []).flatMap((node) => {
    const state = stateById.get(node.id);
    if (!state && node.id !== entryId && node.id !== exitId) {
      return [];
    }
    return [{
      id: node.id,
      state: state ?? (node.id === entryId ? 'entry' : 'exit'),
      x: node.x ?? 0,
      y: node.y ?? 0,
      width: node.width ?? 160,
      height: node.height ?? 44,
      entry: node.id === entryId,
      exit: node.id === exitId,
    }];
  });

  return {
    width: result.width ?? 0,
    height: result.height ?? 0,
    states,
    transitions: (result.edges ?? []).flatMap((edge) =>
      edge.sections?.length
        ? [{
            id: edge.id,
            source: edge.sources[0]!,
            target: edge.targets[0]!,
            sections: positionEdgeSections(edge, new Map()),
          }]
        : [],
    ),
  };
}
