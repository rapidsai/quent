// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { Path, Schema } from '@quent/schema';
import type { Edge, Node } from '@xyflow/svelte';
import type { Component } from 'svelte';

export type SchemaPath = Path;

export interface EntityReference {
  id: string;
  source: SchemaPath;
  target: SchemaPath | null;
  event: string;
  fieldPath: string[];
  tree: boolean;
}

export interface EntityGraphNode {
  id: string;
  path: SchemaPath;
  eventCount: number;
  referenceCount: number;
  fsm: boolean;
  resource: boolean;
}

export interface EntityGraphModel {
  nodes: EntityGraphNode[];
  references: EntityReference[];
}

export interface EntityNodeProps {
  schema: Schema;
  path: SchemaPath;
}

export type EntityNodeComponent = Component<EntityNodeProps>;

export interface EntityGraphConfig {
  direction?: 'down' | 'right' | 'up' | 'left';
  edgeRouting?: 'polyline' | 'orthogonal';
  density?: 'compact' | 'comfortable' | 'spacious';
  layeringStrategy?:
    | 'network-simplex'
    | 'longest-path'
    | 'longest-path-source'
    | 'coffman-graham';
  nodePlacementStrategy?:
    | 'brandes-koepf'
    | 'network-simplex'
    | 'linear-segments'
    | 'simple';
  hierarchicalGreedySwitch?: boolean;
  layoutThoroughness?: number;
  highDegreeNodeTreatment?: boolean;
  groupNamespaces?: boolean;
  references?: 'all' | 'untyped' | 'typed' | 'tree';
  referenceLabels?: 'always' | 'interaction' | 'never';
  showNodeMetadata?: boolean;
  showViewSwitcher?: boolean;
  fitPadding?: number;
  minZoom?: number;
  maxZoom?: number;
  nodeWidth?: number;
  nodeHeight?: number;
}

export interface ResolvedEntityGraphConfig {
  direction: NonNullable<EntityGraphConfig['direction']>;
  edgeRouting: NonNullable<EntityGraphConfig['edgeRouting']>;
  density: NonNullable<EntityGraphConfig['density']>;
  layeringStrategy: NonNullable<EntityGraphConfig['layeringStrategy']>;
  nodePlacementStrategy: NonNullable<
    EntityGraphConfig['nodePlacementStrategy']
  >;
  hierarchicalGreedySwitch: boolean;
  layoutThoroughness: number;
  highDegreeNodeTreatment: boolean;
  groupNamespaces: boolean;
  references: NonNullable<EntityGraphConfig['references']>;
  referenceLabels: NonNullable<EntityGraphConfig['referenceLabels']>;
  showNodeMetadata: boolean;
  showViewSwitcher: boolean;
  fitPadding: number;
  minZoom: number;
  maxZoom: number;
  nodeWidth: number;
  nodeHeight: number;
}

export interface EntityGraphLayoutStart {
  nodeCount: number;
  referenceCount: number;
}

export interface EntityGraphLayoutComplete extends EntityGraphLayoutStart {
  width: number;
  height: number;
  durationMs: number;
}

export interface EntityGraphLayoutError extends EntityGraphLayoutStart {
  error: unknown;
}

export const ENTITY_GRAPH_VIEWS = [
  { id: 'graph', label: 'Entity graph' },
  { id: 'resource-timeline', label: 'Resource timeline' },
] as const;

export type EntityGraphView = (typeof ENTITY_GRAPH_VIEWS)[number]['id'];

export interface EntityGraphViewChange {
  view: EntityGraphView;
}

export interface FsmTransition {
  source: string;
  target: string;
}

export interface FsmTopology {
  initialState: string;
  transitions: FsmTransition[];
  exitStates: string[];
  states: string[];
}

export interface ResourceCapacity {
  name: string;
  kind: 'occupancy' | 'rate';
  bounded: boolean;
}

export interface ResourceConsumer {
  entity: SchemaPath;
  event: string;
  fieldPath: string[];
}

export interface ResourceRecord {
  record: SchemaPath;
  fields: string[];
  consumers: ResourceConsumer[];
}

export interface ResourceDefinition {
  resource: SchemaPath;
  capacities: ResourceCapacity[];
  usages: ResourceRecord[];
  bounds: ResourceRecord[];
}

export type SchemaSelection =
  | { kind: 'entity'; entity: SchemaPath }
  | { kind: 'reference'; reference: EntityReference }
  | { kind: 'record'; record: SchemaPath }
  | { kind: 'event'; entity: SchemaPath; event: string }
  | { kind: 'fsm-state'; entity: SchemaPath; state: string }
  | { kind: 'resource'; resource: SchemaPath }
  | {
      kind: 'resource-record';
      record: SchemaPath;
      resource: SchemaPath;
      role: 'usage' | 'bounds';
    };

export interface EntityGraphClasses {
  empty?: string;
  viewport?: string;
  controls?: string;
  viewSwitcher?: string;
  viewOption?: string;
  activeViewOption?: string;
  timeline?: string;
  timelineHeader?: string;
  timelineRow?: string;
  timelineTrack?: string;
  timelineSegment?: string;
  node?: string;
  fsmNode?: string;
  resourceNode?: string;
  selectedNode?: string;
  badge?: string;
  entityBadge?: string;
  fsmBadge?: string;
  resourceBadge?: string;
  edge?: string;
  treeEdge?: string;
  selectedEdge?: string;
  namespace?: string;
}

export interface EntityFlowNodeData extends Record<string, unknown> {
  schema: Schema;
  node: EntityGraphNode;
  config: ResolvedEntityGraphConfig;
  nodeComponent: EntityNodeComponent | null;
}

export interface NamespaceFlowNodeData extends Record<string, unknown> {
  label: string;
  path: string[];
}

export interface ElkFlowEdgeData extends Record<string, unknown> {
  path: string | null;
  labelX: number | null;
  labelY: number | null;
}

export interface EntityFlowEdgeData extends ElkFlowEdgeData {
  reference: EntityReference;
}

export type EntityFlowNode =
  | Node<EntityFlowNodeData, 'quent-entity'>
  | Node<NamespaceFlowNodeData, 'quent-namespace'>;

export type EntityFlowEdge = Edge<EntityFlowEdgeData, 'quent-elk'>;

export interface FsmFlowNodeData extends Record<string, unknown> {
  label: string;
  state: string;
  entry: boolean;
  exit: boolean;
  initial: boolean;
}

export type FsmFlowNode = Node<FsmFlowNodeData, 'quent-fsm'>;
export type FsmFlowEdge = Edge<ElkFlowEdgeData, 'quent-elk'>;

export interface SchemaDetailsClasses {
  empty?: string;
  section?: string;
  sectionTitle?: string;
  item?: string;
  selectedItem?: string;
  itemTitle?: string;
  itemMeta?: string;
  field?: string;
  fieldType?: string;
  referenceType?: string;
  referenceBadge?: string;
  referenceLabel?: string;
  referenceTarget?: string;
  fsm?: string;
  fsmState?: string;
  selectedFsmState?: string;
  transition?: string;
  resource?: string;
  capacity?: string;
  badge?: string;
  entityBadge?: string;
  recordBadge?: string;
  fsmBadge?: string;
  resourceBadge?: string;
}

export interface QuentEntityGraphElement extends HTMLElement {
  schema: Schema | null;
  selection: SchemaSelection | null;
  classes: EntityGraphClasses;
  config: EntityGraphConfig;
  nodeComponent: EntityNodeComponent | null;
}

export interface QuentPathDetailsElement extends HTMLElement {
  schema: Schema | null;
  path: SchemaPath | null;
  selection: SchemaSelection | null;
  classes: SchemaDetailsClasses;
}

export interface QuentFsmDetailsElement extends QuentPathDetailsElement {
  isolateState: boolean;
}

declare global {
  interface HTMLElementTagNameMap {
    'quent-entity-graph': QuentEntityGraphElement;
    'quent-entity-events': QuentPathDetailsElement;
    'quent-fsm-details': QuentFsmDetailsElement;
    'quent-record-details': QuentPathDetailsElement;
    'quent-resource-details': QuentPathDetailsElement;
  }

  interface GlobalEventHandlersEventMap {
    'quent-select': CustomEvent<SchemaSelection>;
    'quent-hover': CustomEvent<SchemaSelection>;
    'quent-hover-end': CustomEvent<void>;
    'quent-layout-start': CustomEvent<EntityGraphLayoutStart>;
    'quent-layout-complete': CustomEvent<EntityGraphLayoutComplete>;
    'quent-layout-error': CustomEvent<EntityGraphLayoutError>;
    'quent-view-change': CustomEvent<EntityGraphViewChange>;
  }
}
