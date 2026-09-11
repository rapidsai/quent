// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { StatValue } from './dagTypes';

export interface InspectedInformationItem {
  key: string;
  value: StatValue;
  quantity?: string;
}

export interface InspectedInformationGroup {
  heading: string;
  items: InspectedInformationItem[];
}

export interface InspectedPortRelation {
  portId: string;
  role: string;
}

export interface InspectedOperatorObservation {
  timeSeconds: number;
  kind: string;
  attributes: Array<{ key: string; value: StatValue }>;
  portRelations: InspectedPortRelation[];
}

export interface OperatorSelection {
  readonly label: string;
  readonly operatorIds: ReadonlySet<string>;
}

export interface OperatorSelectionInput extends OperatorSelection {
  readonly selectionId: string;
}

export interface OperatorSelectionState {
  readonly selections: ReadonlyMap<string, OperatorSelection>;
  readonly activeId: string | null;
}

export interface InspectedOperatorData {
  nodeId: string;
  label: string;
  operationType: string;
  statistics: InspectedInformationItem[];
  information?: InspectedInformationGroup[];
  observations?: InspectedOperatorObservation[];
  portRelations?: InspectedPortRelation[];
}

export interface InspectedNodeData extends InspectedOperatorData {
  relatedOperators?: InspectedOperatorData[];
}
