// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import {
  defineViewStateVersion,
  readViewStateField,
  readViewStateFields,
  type ViewStateFields,
} from './viewState.fields';
import { ViewStateV1Schema, ViewStateV2Schema, ViewStateV3Schema } from './viewState.schema';

const absentFields = {
  route: undefined,
  timeline: undefined,
  zoomRange: undefined,
  expandedResourceIds: undefined,
  selection: undefined,
  resources: undefined,
  dag: undefined,
  dataFlow: undefined,
  operatorTable: undefined,
  entities: undefined,
} satisfies ViewStateFields;

const v1State = ViewStateV1Schema.parse({
  zoomRange: { start: 1, end: 2 },
  expandedResourceIds: ['b', 'a'],
});

const v2State = ViewStateV2Schema.parse({
  route: { engineId: 'engine-a', queryId: 'query-a', tab: 'timeline' },
  timeline: { zoomRange: { start: 10, end: 40 } },
  resources: { expandedRowIds: ['resource-b', 'resource-a'] },
  selection: { planId: 'plan-a' },
});

const v3State = ViewStateV3Schema.parse({
  route: { engineId: 'engine-a', queryId: 'query-a', tab: 'entities' },
  selection: { operatorNodeIds: ['operator-a'] },
  entities: {
    entityType: 'task',
    window: { start: 12, end: 24 },
    sortDir: 'Asc',
  },
});

describe('view-state field readers', () => {
  it('returns v1 values and omits fields that schema does not own', () => {
    const decoded = { version: 'v1' as const, data: v1State };

    expect(readViewStateField(decoded, 'zoomRange')).toEqual({ start: 1, end: 2 });
    expect(readViewStateField(decoded, 'expandedResourceIds')).toEqual(['a', 'b']);
    expect(readViewStateField(decoded, 'route')).toBeUndefined();
    expect(readViewStateFields(decoded)).toEqual({
      ...absentFields,
      zoomRange: { start: 1, end: 2 },
      expandedResourceIds: ['a', 'b'],
    });
  });

  it('reads v2 nested fields through the same accessors', () => {
    const decoded = { version: 'v2' as const, data: v2State };

    expect(readViewStateField(decoded, 'route')).toEqual(v2State.route);
    expect(readViewStateField(decoded, 'zoomRange')).toEqual({ start: 10, end: 40 });
    expect(readViewStateField(decoded, 'expandedResourceIds')).toEqual([
      'resource-a',
      'resource-b',
    ]);
    expect(readViewStateFields(decoded)).toEqual({
      ...absentFields,
      route: v2State.route,
      timeline: v2State.timeline,
      zoomRange: { start: 10, end: 40 },
      expandedResourceIds: ['resource-a', 'resource-b'],
      selection: { planId: 'plan-a' },
      resources: v2State.resources,
    });
  });

  it('reads v3 entity fields without requiring a timeline viewport', () => {
    const decoded = { version: 'v3' as const, data: v3State };

    expect(readViewStateFields(decoded)).toEqual({
      ...absentFields,
      route: v3State.route,
      selection: v3State.selection,
      entities: v3State.entities,
    });
  });

  it('lets a new version omit fields by not declaring readers', () => {
    const v3 = defineViewStateVersion({
      version: 'v3',
      schema: ViewStateV1Schema,
      fields: {
        zoomRange: state => state.zoomRange,
      },
    });

    expect(v3.readField(v1State, 'zoomRange')).toEqual({ start: 1, end: 2 });
    expect(v3.readField(v1State, 'expandedResourceIds')).toBeUndefined();
    expect(v3.readField(v1State, 'route')).toBeUndefined();
  });
});
