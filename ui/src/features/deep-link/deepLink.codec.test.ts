// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { gzipSync, strToU8 } from 'fflate';
import {
  buildDeepLinkUrl,
  CURRENT_DEEP_LINK_VERSION,
  decodeDeepLinkState,
  encodeDeepLinkState,
  MAX_DEEP_LINK_URL_LENGTH,
  SUPPORTED_DEEP_LINK_VERSIONS,
} from './deepLink.codec';
import {
  DeepLinkStateV1Schema,
  DeepLinkStateV2Schema,
  DeepLinkStateV3Schema,
  MAX_EXPANDED_RESOURCE_IDS,
  OperatorGroupSchema,
  type DeepLinkStateV3,
  validateDeepLinkSearch,
} from './deepLink.schema';
import { CONTINUOUS_PALETTES, DAG_LAYOUT_DIRECTION, NODE_LABEL_FIELD } from '@quent/utils';
import { OPERATOR_TABLE_INDEX_ORDER } from '@/components/operator-table/types';

const RESOURCE_A_ID = '01a025ff-ea8b-7881-9d31-72a275872c9d';
const RESOURCE_B_ID = '01a025ff-ea8b-7881-9d31-72a275872c9e';
const NVTX_SECTION_ID = '__nvtx__';

const state: DeepLinkStateV3 = {
  route: {
    engineId: 'engine-a',
    queryId: 'query-a',
    tab: 'entities',
  },
  timeline: {
    zoomRange: {
      start: 12.5,
      end: 48.75,
    },
  },
  selection: {
    planId: 'plan-a',
    operatorNodeIds: ['operator-a', 'operator-b'],
  },
  resources: {
    expandedRowIds: ['resource-a', 'resource-b'],
    rootResourceType: 'memory',
    resourceTypeSelections: [{ rowId: 'resource-a', resourceType: 'channel' }],
    fsmSelections: [{ rowId: 'resource-a', fsmType: 'task' }],
  },
  dag: {
    nodeColorField: 'duration_s',
    nodeColorPalette: 'viridis',
    edgeWidthField: 'bytes',
    edgeColorField: 'rows',
    edgeColorPalette: 'purple',
    nodeLabelField: 'type',
    layoutDirection: 'top-to-bottom',
  },
  dataFlow: {
    enabled: false,
    measure: 'bytes',
    labelMeasure: 'tasks',
    dimensions: ['filesystem'],
    playheadS: 18,
  },
  operatorTable: {
    groupingOrder: ['partition', 'item_type', 'item'],
    enabledGroups: ['partition', 'item_type'],
    visibleStats: ['duration_s', 'spill_bytes'],
    aggregation: 'max',
    sort: [{ id: 'spill_bytes', desc: true }],
  },
  entities: {
    operatorId: null,
    entityType: 'task',
    resourceId: 'resource-a',
    minUsageS: 0.25,
    window: { start: 10, end: 45 },
    sortDir: 'Asc',
    pageSize: 100,
    page: 2,
    selectedEntityId: 'entity-a',
  },
};

describe('deep-link codec', () => {
  it('round-trips deterministically', () => {
    const first = encodeDeepLinkState(state);
    const second = encodeDeepLinkState(state);

    expect(first).toEqual(second);
    expect(first.ok).toBe(true);
    if (!first.ok) {
      return;
    }
    expect(decodeDeepLinkState(first.value)).toEqual({
      ok: true,
      value: { version: 'v3', data: state },
    });
  });

  it('keeps the current version in the supported schema registry', () => {
    expect(SUPPORTED_DEEP_LINK_VERSIONS).toContain(CURRENT_DEEP_LINK_VERSION);
    expect(new Set(SUPPORTED_DEEP_LINK_VERSIONS).size).toBe(SUPPORTED_DEEP_LINK_VERSIONS.length);
  });

  it('rejects unsupported versions and malformed payloads', () => {
    expect(decodeDeepLinkState('v999.abc')).toMatchObject({
      ok: false,
      code: 'unsupported-version',
    });
    expect(decodeDeepLinkState('v2.not-gzip')).toMatchObject({
      ok: false,
      code: 'invalid-encoding',
    });
  });

  it('decodes legacy v1 links', () => {
    const encoded = 'v1.H4sIAAAAAAACA6tWqsrPzw1KzEtPVbKqViouSSwqUbIy0FFKzUsB0nomBua1tQAidcVYJQAAAA';

    expect(decodeDeepLinkState(encoded)).toEqual({
      ok: true,
      value: { version: 'v1', data: { zoomRange: { start: 0, end: 0.407 } } },
    });
  });

  it('decodes legacy v2 links', () => {
    const legacyState = DeepLinkStateV2Schema.parse({
      route: { engineId: 'engine-a', queryId: 'query-a', tab: 'operators' },
      timeline: { zoomRange: { start: 12.5, end: 48.75 } },
      selection: { operatorNodeIds: ['operator-a'] },
    });
    const compressed = gzipSync(strToU8(JSON.stringify(legacyState)), { level: 9, mtime: 0 });
    let binary = '';
    for (const byte of compressed) {
      binary += String.fromCharCode(byte);
    }
    const encoded = `v2.${btoa(binary)
      .replace(/\+/gu, '-')
      .replace(/\//gu, '_')
      .replace(/=+$/u, '')}`;

    expect(decodeDeepLinkState(encoded)).toEqual({
      ok: true,
      value: { version: 'v2', data: legacyState },
    });
  });

  it('builds links within the absolute URL budget', () => {
    const result = buildDeepLinkUrl(
      'https://quent.example.test/profile/engine/e/query/q/timeline',
      state
    );

    expect(result.ok).toBe(true);
    if (!result.ok) {
      return;
    }
    expect(result.value.length).toBeLessThanOrEqual(MAX_DEEP_LINK_URL_LENGTH);
    expect(new URL(result.value).searchParams.get('s')).toMatch(
      new RegExp(`^${CURRENT_DEEP_LINK_VERSION}\\.`)
    );
  });

  it('rejects an absolute URL whose origin and path exhaust the budget', () => {
    const result = buildDeepLinkUrl(
      `https://quent.example.test/${'x'.repeat(MAX_DEEP_LINK_URL_LENGTH)}`,
      state
    );

    expect(result).toMatchObject({ ok: false, code: 'url-too-long' });
  });
});

describe('deep-link search validation', () => {
  it('strips unknown keys and rejects non-string state', () => {
    expect(validateDeepLinkSearch({ s: 'v1.abc', extra: true })).toEqual({ s: 'v1.abc' });
    expect(validateDeepLinkSearch({ s: 42 })).toEqual({});
  });
});

describe('deep-link state validation', () => {
  it('validates and canonicalizes comprehensive v3 state', () => {
    expect(DeepLinkStateV3Schema.parse({ ...state, futureField: true })).toEqual(state);
    expect(
      DeepLinkStateV3Schema.parse({
        route: state.route,
        timeline: state.timeline,
        resources: {
          expandedRowIds: [RESOURCE_B_ID, NVTX_SECTION_ID, RESOURCE_A_ID, RESOURCE_B_ID],
        },
      }).resources?.expandedRowIds
    ).toEqual([RESOURCE_A_ID, RESOURCE_B_ID, NVTX_SECTION_ID]);
    expect(
      DeepLinkStateV3Schema.safeParse({
        route: state.route,
        timeline: { zoomRange: { start: 20, end: 10 } },
      }).success
    ).toBe(false);
    expect(
      DeepLinkStateV3Schema.safeParse({
        route: state.route,
        timeline: state.timeline,
        resources: {
          expandedRowIds: Array.from(
            { length: MAX_EXPANDED_RESOURCE_IDS + 1 },
            (_, index) => `resource-${index}`
          ),
        },
      }).success
    ).toBe(false);
    expect(
      DeepLinkStateV3Schema.safeParse({
        route: state.route,
        entities: { window: { start: 20, end: 10 } },
      }).success
    ).toBe(false);
  });

  it('keeps the v2 schema unchanged', () => {
    expect(
      DeepLinkStateV2Schema.safeParse({
        route: { engineId: 'engine-a', queryId: 'query-a', tab: 'entities' },
        timeline: state.timeline,
      }).success
    ).toBe(false);
  });

  it('keeps the legacy v1 schema available for old links', () => {
    expect(
      DeepLinkStateV1Schema.parse({
        zoomRange: { start: 1, end: 2 },
        expandedResourceIds: ['b', 'a'],
      })
    ).toEqual({
      zoomRange: { start: 1, end: 2 },
      expandedResourceIds: ['a', 'b'],
    });
    expect(
      DeepLinkStateV1Schema.parse({
        zoomRange: { start: 1, end: 2 },
        expandedResourceIds: [RESOURCE_B_ID, NVTX_SECTION_ID, RESOURCE_A_ID, RESOURCE_B_ID],
      }).expandedResourceIds
    ).toEqual([RESOURCE_A_ID, RESOURCE_B_ID, NVTX_SECTION_ID]);
    expect(DeepLinkStateV1Schema.safeParse({ zoomRange: { start: 20, end: 10 } }).success).toBe(
      false
    );
    expect(
      DeepLinkStateV1Schema.safeParse({
        zoomRange: { start: 1, end: 2 },
        expandedResourceIds: [''],
      }).success
    ).toBe(false);
  });

  it('derives enum vocabularies from shared constants', () => {
    const base = { route: state.route, timeline: state.timeline };
    for (const palette of Object.keys(CONTINUOUS_PALETTES)) {
      expect(
        DeepLinkStateV3Schema.safeParse({
          ...base,
          dag: { nodeColorPalette: palette, edgeColorPalette: palette },
        }).success
      ).toBe(true);
    }
    for (const nodeLabelField of Object.values(NODE_LABEL_FIELD)) {
      expect(DeepLinkStateV3Schema.safeParse({ ...base, dag: { nodeLabelField } }).success).toBe(
        true
      );
    }
    for (const layoutDirection of Object.values(DAG_LAYOUT_DIRECTION)) {
      expect(DeepLinkStateV3Schema.safeParse({ ...base, dag: { layoutDirection } }).success).toBe(
        true
      );
    }
    expect(OperatorGroupSchema.options).toEqual([...OPERATOR_TABLE_INDEX_ORDER]);
  });
});
