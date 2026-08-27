// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import {
  defineDeepLinkVersion,
  readDeepLinkField,
  readDeepLinkFields,
  type DeepLinkFields,
} from './deepLink.fields';
import { DeepLinkStateV1Schema, DeepLinkStateV2Schema } from './deepLink.schema';

const absentFields = {
  route: undefined,
  zoomRange: undefined,
  expandedResourceIds: undefined,
  selection: undefined,
  resources: undefined,
  dag: undefined,
  dataFlow: undefined,
  operatorTable: undefined,
} satisfies DeepLinkFields;

const v1State = DeepLinkStateV1Schema.parse({
  zoomRange: { start: 1, end: 2 },
  expandedResourceIds: ['b', 'a'],
});

const v2State = DeepLinkStateV2Schema.parse({
  route: { engineId: 'engine-a', queryId: 'query-a', tab: 'timeline' },
  timeline: { zoomRange: { start: 10, end: 40 } },
  resources: { expandedRowIds: ['resource-b', 'resource-a'] },
  selection: { planId: 'plan-a' },
});

describe('deep-link field readers', () => {
  it('returns v1 values and omits fields that schema does not own', () => {
    const decoded = { version: 'v1' as const, data: v1State };

    expect(readDeepLinkField(decoded, 'zoomRange')).toEqual({ start: 1, end: 2 });
    expect(readDeepLinkField(decoded, 'expandedResourceIds')).toEqual(['a', 'b']);
    expect(readDeepLinkField(decoded, 'route')).toBeUndefined();
    expect(readDeepLinkFields(decoded)).toEqual({
      ...absentFields,
      zoomRange: { start: 1, end: 2 },
      expandedResourceIds: ['a', 'b'],
    });
  });

  it('reads v2 nested fields through the same accessors', () => {
    const decoded = { version: 'v2' as const, data: v2State };

    expect(readDeepLinkField(decoded, 'route')).toEqual(v2State.route);
    expect(readDeepLinkField(decoded, 'zoomRange')).toEqual({ start: 10, end: 40 });
    expect(readDeepLinkField(decoded, 'expandedResourceIds')).toEqual(['resource-a', 'resource-b']);
    expect(readDeepLinkFields(decoded)).toEqual({
      ...absentFields,
      route: v2State.route,
      zoomRange: { start: 10, end: 40 },
      expandedResourceIds: ['resource-a', 'resource-b'],
      selection: { planId: 'plan-a' },
      resources: v2State.resources,
    });
  });

  it('lets a new version omit fields by not declaring readers', () => {
    const v3 = defineDeepLinkVersion({
      version: 'v3',
      schema: DeepLinkStateV1Schema,
      fields: {
        zoomRange: state => state.zoomRange,
      },
    });

    expect(v3.readField(v1State, 'zoomRange')).toEqual({ start: 1, end: 2 });
    expect(v3.readField(v1State, 'expandedResourceIds')).toBeUndefined();
    expect(v3.readField(v1State, 'route')).toBeUndefined();
  });
});
