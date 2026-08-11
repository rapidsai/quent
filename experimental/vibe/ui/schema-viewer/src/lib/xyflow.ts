// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  MarkerType,
  Position,
  type SvelteFlowProps,
} from '@xyflow/svelte';

import type {
  EntityGraphLayout,
  FsmGraphLayout,
  PositionedEdgePoint,
  PositionedEdgeSection,
} from './layout';
import { referenceLabel } from './schema';
import { selectionMatches } from './selection';
import type {
  EntityFlowEdge,
  EntityFlowNode,
  EntityGraphClasses,
  EntityNodeComponent,
  FsmFlowEdge,
  FsmFlowNode,
  FsmTopology,
  ResolvedEntityGraphConfig,
  SchemaDetailsClasses,
  SchemaPath,
  SchemaSelection,
} from './types';
import type { Schema } from '@quent/schema';

const FLOW_ARROW_MARKER = {
  type: MarkerType.ArrowClosed,
  width: 24,
  height: 24,
  markerUnits: 'userSpaceOnUse',
  strokeWidth: 1.5,
} as const;

export const READ_ONLY_FLOW_CONFIG = {
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
} as const satisfies Partial<SvelteFlowProps>;

export interface EntityFlowElements {
  nodes: EntityFlowNode[];
  edges: EntityFlowEdge[];
}

export interface EntityFlowAdapterInput {
  schema: Schema;
  layout: EntityGraphLayout;
  config: ResolvedEntityGraphConfig;
  classes: EntityGraphClasses;
  selection: SchemaSelection | null;
  nodeComponent: EntityNodeComponent | null;
}

export interface FsmFlowAdapterInput {
  path: SchemaPath;
  topology: FsmTopology;
  layout: FsmGraphLayout;
  selection: SchemaSelection | null;
  classes: SchemaDetailsClasses;
}

export function toEntityFlowElements(
  input: EntityFlowAdapterInput,
): EntityFlowElements {
  const {
    schema,
    layout,
    config,
    classes,
    selection,
    nodeComponent,
  } = input;
  const groups = new Map(layout.groups.map((group) => [group.id, group]));
  const positions = flowPositions(config.direction);
  const groupNodes: EntityFlowNode[] = [...layout.groups]
    .sort((left, right) => left.depth - right.depth)
    .map((group) => {
      const parent = group.parentId ? groups.get(group.parentId) : undefined;
      return {
        id: group.id,
        type: 'quent-namespace',
        position: {
          x: group.x - (parent?.x ?? 0),
          y: group.y - (parent?.y ?? 0),
        },
        parentId: group.parentId ?? undefined,
        initialWidth: group.width,
        initialHeight: group.height,
        style: `width:${group.width}px;height:${group.height}px`,
        data: {
          label: group.label,
          path: group.path,
        },
        selectable: false,
        draggable: false,
        connectable: false,
        deletable: false,
        focusable: false,
        domAttributes: {
          'data-quent-role': 'namespace',
          'data-namespace': group.path.join('::'),
        },
        class: classes.namespace,
      };
    });
  const entityNodes: EntityFlowNode[] = layout.nodes.map((node) => {
    const parent = node.parentId ? groups.get(node.parentId) : undefined;
    const value: SchemaSelection = { kind: 'entity', entity: node.path };
    return {
      id: node.id,
      type: 'quent-entity',
      position: {
        x: node.x - (parent?.x ?? 0),
        y: node.y - (parent?.y ?? 0),
      },
      parentId: node.parentId ?? undefined,
      initialWidth: node.width,
      initialHeight: node.height,
      style: `width:${node.width}px;min-height:${node.height}px`,
      sourcePosition: positions.source,
      targetPosition: positions.target,
      data: {
        schema,
        node,
        config,
        nodeComponent,
      },
      selected: selectionMatches(selection, value),
      selectable: true,
      draggable: false,
      connectable: false,
      deletable: false,
      focusable: true,
      ariaLabel: `Entity ${node.id}`,
      class: [
        'quent-entity-graph__node',
        node.fsm && 'quent-entity-graph__node--fsm',
        node.resource && 'quent-entity-graph__node--resource',
        classes.node,
        node.fsm && classes.fsmNode,
        node.resource && classes.resourceNode,
        selectionMatches(selection, value) && classes.selectedNode,
      ].filter(Boolean).join(' '),
      domAttributes: {
        'data-quent-role': 'entity',
        'data-entity': node.id,
        'data-fsm': String(node.fsm),
        'data-resource': String(node.resource),
        title: node.id,
      },
    };
  });
  const edges = layout.references.flatMap<EntityFlowEdge>(
    ({ id, reference, source, target, sections, labelPosition }) => {
      const selected = selectionMatches(selection, {
        kind: 'reference',
        reference,
      });
      const geometry = edgeGeometryFromSections(sections);
      return [{
        id,
        source,
        target,
        type: 'quent-elk',
        data: {
          reference,
          path: geometry?.path ?? null,
          labelX: labelPosition?.x ?? geometry?.label.x ?? null,
          labelY: labelPosition?.y ?? geometry?.label.y ?? null,
        },
        label: config.referenceLabels === 'never'
          ? undefined
          : referenceLabel(reference),
        selected,
        selectable: true,
        deletable: false,
        focusable: true,
        ariaLabel: `Reference ${referenceLabel(reference)}`,
        interactionWidth: 20,
        markerEnd: {
          ...FLOW_ARROW_MARKER,
          color: selected
            ? 'var(--quent-viewer-accent)'
            : reference.tree
              ? 'var(--quent-viewer-tree)'
              : 'var(--quent-viewer-muted)',
        },
        style: selected
          ? '--xy-edge-stroke-width:4.5'
          : reference.tree
            ? '--xy-edge-stroke:var(--quent-viewer-tree);--xy-edge-stroke-width:4'
            : undefined,
        class: [
          'quent-entity-graph__edge',
          reference.tree && 'quent-entity-graph__edge--tree',
          config.referenceLabels === 'interaction' &&
            'quent-entity-graph__edge--interaction-label',
          selected && 'quent-entity-graph__edge--selected',
          classes.edge,
          reference.tree && classes.treeEdge,
          selected && classes.selectedEdge,
        ].filter(Boolean).join(' '),
        domAttributes: {
          'data-quent-role': 'reference',
          'data-tree': String(reference.tree),
        },
      }];
    },
  );

  return {
    nodes: [...groupNodes, ...entityNodes],
    edges,
  };
}

export function toFsmFlowElements(
  input: FsmFlowAdapterInput,
): { nodes: FsmFlowNode[]; edges: FsmFlowEdge[] } {
  const { path, topology, layout, selection, classes } = input;
  return {
    nodes: layout.states.map((state) => {
      const stateSelection = {
        kind: 'fsm-state',
        entity: path,
        state: state.state,
      } as const;
      const selected =
        !state.entry &&
        !state.exit &&
        selectionMatches(selection, stateSelection);
      return {
        id: state.id,
        type: 'quent-fsm',
        position: { x: state.x, y: state.y },
        initialWidth: state.width,
        initialHeight: state.height,
        style: `width:${state.width}px;height:${state.height}px`,
        data: {
          label: state.entry || state.exit ? '' : state.state,
          state: state.state,
          entry: state.entry,
          exit: state.exit,
          initial: state.state === topology.initialState,
        },
        selected,
        selectable: !state.entry && !state.exit,
        draggable: false,
        connectable: false,
        deletable: false,
        focusable: !state.entry && !state.exit,
        class: [
          'quent-schema-details__fsm-flow-node-wrapper',
          state.entry &&
            'quent-schema-details__fsm-flow-node-wrapper--entry',
          state.exit &&
            'quent-schema-details__fsm-flow-node-wrapper--exit',
          state.state === topology.initialState &&
            'quent-schema-details__fsm-flow-node-wrapper--initial',
          selected &&
            'quent-schema-details__fsm-flow-node-wrapper--selected',
          !state.entry && !state.exit && classes.fsmState,
          selected && classes.selectedFsmState,
        ].filter(Boolean).join(' '),
        domAttributes: {
          'data-quent-role': state.entry
            ? 'fsm-entry'
            : state.exit
              ? 'fsm-exit'
              : 'fsm-state',
          'data-state': state.state,
          'data-quent-schema-name': String(!state.entry && !state.exit),
        },
      };
    }),
    edges: layout.transitions.map((transition) => {
      const geometry = edgeGeometryFromSections(transition.sections);
      return {
        id: transition.id,
        source: transition.source,
        target: transition.target,
        type: 'quent-elk',
        data: {
          path: geometry?.path ?? null,
          labelX: null,
          labelY: null,
        },
        markerEnd: { ...FLOW_ARROW_MARKER },
        selectable: false,
        deletable: false,
        focusable: false,
        class: [
          'quent-schema-details__fsm-flow-edge',
          classes.transition,
        ].filter(Boolean).join(' '),
        domAttributes: {
          'data-quent-role': 'fsm-transition',
        },
      };
    }),
  };
}

export function edgeGeometryFromSections(
  sections: PositionedEdgeSection[],
  cornerRadius = 12,
): { path: string; label: PositionedEdgePoint } | null {
  const paths = sections.flatMap((section) => {
    const points = [
      section.startPoint,
      ...section.bendPoints,
      section.endPoint,
    ];
    return points.length >= 2
      ? [{
          points,
          path: roundedPolylinePath(points, cornerRadius),
        }]
      : [];
  });
  if (paths.length === 0) {
    return null;
  }

  const segments = paths.flatMap(({ points }) =>
    points.slice(1).map((point, index) => {
      const start = points[index]!;
      return {
        start,
        end: point,
        length: Math.hypot(point.x - start.x, point.y - start.y),
      };
    }),
  );
  const longestSegment = segments.reduce((longest, segment) =>
    segment.length > longest.length ? segment : longest
  );
  if (longestSegment) {
    return {
      path: paths.map((value) => value.path).join(' '),
      label: {
        x: (longestSegment.start.x + longestSegment.end.x) / 2,
        y: (longestSegment.start.y + longestSegment.end.y) / 2,
      },
    };
  }

  return {
    path: paths.map((value) => value.path).join(' '),
    label: paths.at(-1)!.points.at(-1)!,
  };
}

function roundedPolylinePath(
  points: PositionedEdgePoint[],
  cornerRadius: number,
): string {
  const first = points[0];
  if (!first) {
    return '';
  }
  if (points.length === 1 || cornerRadius <= 0) {
    return points
      .map((point, index) =>
        `${index === 0 ? 'M' : 'L'} ${point.x} ${point.y}`)
      .join(' ');
  }

  const commands = [`M ${first.x} ${first.y}`];
  for (let index = 1; index < points.length - 1; index += 1) {
    const previous = points[index - 1]!;
    const corner = points[index]!;
    const next = points[index + 1]!;
    const incomingLength = Math.hypot(
      corner.x - previous.x,
      corner.y - previous.y,
    );
    const outgoingLength = Math.hypot(
      next.x - corner.x,
      next.y - corner.y,
    );
    const radius = Math.min(
      cornerRadius,
      incomingLength / 2,
      outgoingLength / 2,
    );
    if (radius === 0) {
      commands.push(`L ${corner.x} ${corner.y}`);
      continue;
    }
    const entry = {
      x: corner.x - ((corner.x - previous.x) / incomingLength) * radius,
      y: corner.y - ((corner.y - previous.y) / incomingLength) * radius,
    };
    const exit = {
      x: corner.x + ((next.x - corner.x) / outgoingLength) * radius,
      y: corner.y + ((next.y - corner.y) / outgoingLength) * radius,
    };
    commands.push(
      `L ${entry.x} ${entry.y} Q ${corner.x} ${corner.y} ${exit.x} ${exit.y}`,
    );
  }
  const last = points.at(-1)!;
  commands.push(`L ${last.x} ${last.y}`);
  return commands.join(' ');
}

function flowPositions(
  direction: ResolvedEntityGraphConfig['direction'],
): { source: Position; target: Position } {
  switch (direction) {
    case 'up':
      return { source: Position.Top, target: Position.Bottom };
    case 'left':
      return { source: Position.Left, target: Position.Right };
    case 'right':
      return { source: Position.Right, target: Position.Left };
    default:
      return { source: Position.Bottom, target: Position.Top };
  }
}

export function entitySelectionFromFlowNode(
  node: EntityFlowNode,
): SchemaSelection | null {
  return node.type === 'quent-entity'
    ? { kind: 'entity', entity: node.data.node.path }
    : null;
}

export function referenceSelectionFromFlowEdge(
  edge: EntityFlowEdge,
): SchemaSelection {
  return { kind: 'reference', reference: edge.data!.reference };
}
