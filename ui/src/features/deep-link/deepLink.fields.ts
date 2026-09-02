// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { ZoomRange } from '@quent/utils';
import { z } from 'zod';
import {
  DeepLinkStateV1Schema,
  DeepLinkStateV2Schema,
  DeepLinkStateV3Schema,
  type DeepLinkStateV1,
  type DeepLinkStateV2,
  type DeepLinkStateV3,
} from './deepLink.schema';

/** Canonical fields a decoded link may expose. Missing readers return `undefined`. */
export type DeepLinkFields = {
  route: DeepLinkStateV3['route'] | undefined;
  zoomRange: ZoomRange | undefined;
  expandedResourceIds: readonly string[] | undefined;
  selection: DeepLinkStateV3['selection'];
  resources: DeepLinkStateV3['resources'];
  dag: DeepLinkStateV3['dag'];
  dataFlow: DeepLinkStateV3['dataFlow'];
  operatorTable: DeepLinkStateV3['operatorTable'];
  entities: DeepLinkStateV3['entities'];
};

export type DeepLinkFieldKey = keyof DeepLinkFields;

export type DeepLinkFieldReaders<TState> = {
  [K in DeepLinkFieldKey]?: (state: TState) => DeepLinkFields[K];
};

type DeepLinkVersionEntry<V extends string, TState> = {
  version: V;
  schema: z.ZodType<TState>;
  readField: <K extends DeepLinkFieldKey>(data: unknown, field: K) => DeepLinkFields[K];
  readFields: (data: unknown) => DeepLinkFields;
};

function readFields<TState>(readers: DeepLinkFieldReaders<TState>, state: TState): DeepLinkFields {
  return {
    route: readers.route?.(state),
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

export function defineDeepLinkVersion<V extends string, TState>(definition: {
  version: V;
  schema: z.ZodType<TState>;
  fields: DeepLinkFieldReaders<TState>;
}): DeepLinkVersionEntry<V, TState> {
  return {
    version: definition.version,
    schema: definition.schema,
    readField: (data, field) => {
      const reader = definition.fields[field];
      return reader ? reader(data as TState) : (undefined as DeepLinkFields[typeof field]);
    },
    readFields: data => readFields(definition.fields, data as TState),
  };
}

export const SUPPORTED_DEEP_LINK_SCHEMAS = [
  defineDeepLinkVersion({
    version: 'v1',
    schema: DeepLinkStateV1Schema,
    fields: {
      zoomRange: state => state.zoomRange,
      expandedResourceIds: state => state.expandedResourceIds,
    },
  }),
  defineDeepLinkVersion({
    version: 'v2',
    schema: DeepLinkStateV2Schema,
    fields: {
      route: state => state.route,
      zoomRange: state => state.timeline.zoomRange,
      expandedResourceIds: state => state.resources?.expandedRowIds,
      selection: state => state.selection,
      resources: state => state.resources,
      dag: state => state.dag,
      dataFlow: state => state.dataFlow,
      operatorTable: state => state.operatorTable,
    },
  }),
  defineDeepLinkVersion({
    version: 'v3',
    schema: DeepLinkStateV3Schema,
    fields: {
      route: state => state.route,
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

export type DeepLinkVersion = (typeof SUPPORTED_DEEP_LINK_SCHEMAS)[number]['version'];
export type DecodedDeepLinkState = DeepLinkStateV1 | DeepLinkStateV2 | DeepLinkStateV3;

export type VersionedDeepLinkState = {
  version: DeepLinkVersion;
  data: DecodedDeepLinkState;
};

function schemaFor(version: DeepLinkVersion) {
  const entry = SUPPORTED_DEEP_LINK_SCHEMAS.find(item => item.version === version);
  if (!entry) {
    throw new Error(`Unsupported deep-link version: ${version}`);
  }
  return entry;
}

export function readDeepLinkField<K extends DeepLinkFieldKey>(
  decoded: VersionedDeepLinkState,
  field: K
): DeepLinkFields[K] {
  return schemaFor(decoded.version).readField(decoded.data, field);
}

export function readDeepLinkFields(decoded: VersionedDeepLinkState): DeepLinkFields {
  return schemaFor(decoded.version).readFields(decoded.data);
}
