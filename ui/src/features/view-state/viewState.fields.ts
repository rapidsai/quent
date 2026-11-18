// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { ZoomRange } from '@quent/utils';
import { z } from 'zod';
import {
  ViewStateV1Schema,
  ViewStateV2Schema,
  ViewStateV3Schema,
  type ViewStateV1,
  type ViewStateV2,
  type ViewStateV3,
} from './viewState.schema';

/** Canonical fields a decoded link may expose. Missing readers return `undefined`. */
export type ViewStateFields = {
  route: ViewStateV3['route'] | undefined;
  timeline: ViewStateV3['timeline'];
  zoomRange: ZoomRange | undefined;
  expandedResourceIds: readonly string[] | undefined;
  selection: ViewStateV3['selection'];
  resources: ViewStateV3['resources'];
  dag: ViewStateV3['dag'];
  dataFlow: ViewStateV3['dataFlow'];
  operatorTable: ViewStateV3['operatorTable'];
  entities: ViewStateV3['entities'];
};

export type ViewStateFieldKey = keyof ViewStateFields;

export type ViewStateFieldReaders<TState> = {
  [K in ViewStateFieldKey]?: (state: TState) => ViewStateFields[K];
};

type ViewStateVersionEntry<V extends string, TState> = {
  version: V;
  schema: z.ZodType<TState>;
  readField: <K extends ViewStateFieldKey>(data: unknown, field: K) => ViewStateFields[K];
  readFields: (data: unknown) => ViewStateFields;
};

function readFields<TState>(
  readers: ViewStateFieldReaders<TState>,
  state: TState
): ViewStateFields {
  return {
    route: readers.route?.(state),
    timeline: readers.timeline?.(state),
    zoomRange: readers.zoomRange?.(state),
    expandedResourceIds: readers.expandedResourceIds?.(state),
    selection: readers.selection?.(state),
    resources: readers.resources?.(state),
    dag: readers.dag?.(state),
    dataFlow: readers.dataFlow?.(state),
    operatorTable: readers.operatorTable?.(state),
    entities: readers.entities?.(state),
  };
}

export function defineViewStateVersion<V extends string, TState>(definition: {
  version: V;
  schema: z.ZodType<TState>;
  fields: ViewStateFieldReaders<TState>;
}): ViewStateVersionEntry<V, TState> {
  return {
    version: definition.version,
    schema: definition.schema,
    readField: (data, field) => {
      const reader = definition.fields[field];
      return reader ? reader(data as TState) : (undefined as ViewStateFields[typeof field]);
    },
    readFields: data => readFields(definition.fields, data as TState),
  };
}

export const SUPPORTED_VIEW_STATE_SCHEMAS = [
  defineViewStateVersion({
    version: 'v1',
    schema: ViewStateV1Schema,
    fields: {
      zoomRange: state => state.zoomRange,
      expandedResourceIds: state => state.expandedResourceIds,
    },
  }),
  defineViewStateVersion({
    version: 'v2',
    schema: ViewStateV2Schema,
    fields: {
      route: state => state.route,
      timeline: state => state.timeline,
      zoomRange: state => state.timeline.zoomRange,
      expandedResourceIds: state => state.resources?.expandedRowIds,
      selection: state => state.selection,
      resources: state => state.resources,
      dag: state => state.dag,
      dataFlow: state => state.dataFlow,
      operatorTable: state => state.operatorTable,
    },
  }),
  defineViewStateVersion({
    version: 'v3',
    schema: ViewStateV3Schema,
    fields: {
      route: state => state.route,
      timeline: state => state.timeline,
      zoomRange: state => state.timeline?.zoomRange,
      expandedResourceIds: state => state.resources?.expandedRowIds,
      selection: state => state.selection,
      resources: state => state.resources,
      dag: state => state.dag,
      dataFlow: state => state.dataFlow,
      operatorTable: state => state.operatorTable,
      entities: state => state.entities,
    },
  }),
] as const;

export type ViewStateVersion = (typeof SUPPORTED_VIEW_STATE_SCHEMAS)[number]['version'];
export type DecodedViewState = ViewStateV1 | ViewStateV2 | ViewStateV3;

export type VersionedViewState = {
  version: ViewStateVersion;
  data: DecodedViewState;
};

function schemaFor(version: ViewStateVersion) {
  const entry = SUPPORTED_VIEW_STATE_SCHEMAS.find(item => item.version === version);
  if (!entry) {
    throw new Error(`Unsupported view-state version: ${version}`);
  }
  return entry;
}

export function readViewStateField<K extends ViewStateFieldKey>(
  decoded: VersionedViewState,
  field: K
): ViewStateFields[K] {
  return schemaFor(decoded.version).readField(decoded.data, field);
}

export function readViewStateFields(decoded: VersionedViewState): ViewStateFields {
  return schemaFor(decoded.version).readFields(decoded.data);
}
