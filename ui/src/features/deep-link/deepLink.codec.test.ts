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
  MAX_SELECTED_NODE_IDS,
  OperatorGroupSchema,
  type DeepLinkStateV3,
  validateDeepLinkSearch,
} from './deepLink.schema';
import {
  statisticFieldId,
  CONTINUOUS_PALETTES,
  DAG_LAYOUT_DIRECTION,
  SCALE_TYPE,
  NODE_LABEL_FIELD,
} from '@quent/utils';
import { OPERATOR_TABLE_INDEX_ORDER } from '@/components/operator-table/types';
import { MAX_RESOURCE_FILTER_QUERY_LENGTH } from '@/features/resource-filter/resourceFilter';
import { MAX_PINNED_ROWS } from '@/features/pinned-rows/pinnedRows';

const RESOURCE_A_ID = '01a025ff-ea8b-7881-9d31-72a275872c9d';
const RESOURCE_B_ID = '01a025ff-ea8b-7881-9d31-72a275872c9e';
const NVTX_SECTION_ID = '__nvtx__';

function encodeRawState(version: string, value: unknown): string {
  const compressed = gzipSync(strToU8(JSON.stringify(value)), { level: 9, mtime: 0 });
  let binary = '';
  for (const byte of compressed) {
    binary += String.fromCharCode(byte);
  }
  return `${version}.${btoa(binary).replace(/\+/gu, '-').replace(/\//gu, '_').replace(/=+$/u, '')}`;
}

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
    resourceFilter: {
      search: 'resource',
      resourceTypes: ['channel', 'memory'],
      fsmTypes: ['task', 'transfer'],
      showOthers: true,
    },
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
    scaleType: 'linear',
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

  it('round-trips explicit statistic paths longer than the generic name limit', () => {
    const fieldId = statisticFieldId([
      ['Volume '.repeat(100), 0],
      ['bytes '.repeat(100), 2],
    ]);
    const nestedState: DeepLinkStateV3 = {
      ...state,
      dag: {
        ...state.dag,
        nodeColorField: fieldId,
        edgeWidthField: fieldId,
        edgeColorField: fieldId,
      },
    };
    const encoded = encodeDeepLinkState(nestedState);
    expect(encoded.ok).toBe(true);
    if (!encoded.ok) {
      throw new Error('Failed to encode nested statistic selections');
    }
    expect(decodeDeepLinkState(encoded.value)).toEqual({
      ok: true,
      value: { version: 'v3', data: nestedState },
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
    const encoded = encodeRawState('v2', legacyState);

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

  it('reports the selected operator limit when encoding', () => {
    const atLimit = Array.from(
      { length: MAX_SELECTED_NODE_IDS },
      (_, index) => `operator-${index}`
    );
    expect(
      DeepLinkStateV3Schema.safeParse({
        route: state.route,
        selection: { operatorNodeIds: atLimit },
      }).success
    ).toBe(true);

    expect(
      encodeDeepLinkState({
        route: state.route,
        selection: { operatorNodeIds: [...atLimit, 'operator-over-limit'] },
      })
    ).toEqual({
      ok: false,
      code: 'invalid-state',
      message: `Shared links support at most ${MAX_SELECTED_NODE_IDS} selected operators.`,
      messages: [`Shared links support at most ${MAX_SELECTED_NODE_IDS} selected operators.`],
    });
  });

  it('reports the applicable limit for other oversized state', () => {
    const expandedRowIds = Array.from(
      { length: MAX_EXPANDED_RESOURCE_IDS + 1 },
      (_, index) => `resource-${index}`
    );

    expect(
      encodeDeepLinkState({
        route: state.route,
        resources: { expandedRowIds },
      })
    ).toEqual({
      ok: false,
      code: 'invalid-state',
      message: `Shared links support at most ${MAX_EXPANDED_RESOURCE_IDS} expanded resources.`,
      messages: [`Shared links support at most ${MAX_EXPANDED_RESOURCE_IDS} expanded resources.`],
    });
  });

  it('reports every limit violation when encoding and decoding', () => {
    const selectedMessage = `Shared links support at most ${MAX_SELECTED_NODE_IDS} selected operators.`;
    const expandedMessage = `Shared links support at most ${MAX_EXPANDED_RESOURCE_IDS} expanded resources.`;
    const invalidState = {
      route: state.route,
      selection: {
        operatorNodeIds: Array.from(
          { length: MAX_SELECTED_NODE_IDS + 1 },
          (_, index) => `operator-${index}`
        ),
      },
      resources: {
        expandedRowIds: Array.from(
          { length: MAX_EXPANDED_RESOURCE_IDS + 1 },
          (_, index) => `resource-${index}`
        ),
      },
    };
    const expected = {
      ok: false,
      code: 'invalid-state',
      message: `${selectedMessage} ${expandedMessage}`,
      messages: [selectedMessage, expandedMessage],
    };

    expect(encodeDeepLinkState(invalidState)).toEqual(expected);
    expect(decodeDeepLinkState(encodeRawState('v3', invalidState))).toEqual(expected);
  });

  it('includes nested fields and array indexes in validation errors', () => {
    const invalidState = {
      ...state,
      timeline: { zoomRange: { start: -1, end: 0 } },
      resources: {
        ...state.resources,
        resourceTypeSelections: [{ rowId: 'resource-a', resourceType: '' }],
      },
      entities: { ...state.entities, window: { start: 20, end: 10 } },
    };

    expect(encodeDeepLinkState(invalidState)).toMatchObject({
      ok: false,
      code: 'invalid-state',
      messages: [
        'Too small: expected number to be >=0 (timeline.zoomRange.start)',
        'Too small: expected number to be >0 (timeline.zoomRange.end)',
        'Too small: expected string to have >=1 characters (resources.resourceTypeSelections[0].resourceType)',
        'Entity window end must not precede its start',
      ],
    });
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
      DeepLinkStateV3Schema.validate({
        route: state.route,
        timeline: { zoomRange: { start: 20, end: 10 } },
      })
    ).toBe(false);
    expect(
      DeepLinkStateV3Schema.validate({
        route: state.route,
        timeline: state.timeline,
        resources: {
          expandedRowIds: Array.from(
            { length: MAX_EXPANDED_RESOURCE_IDS + 1 },
            (_, index) => `resource-${index}`
          ),
        },
      })
    ).toBe(false);
    expect(
      DeepLinkStateV3Schema.validate({
        route: state.route,
        timeline: state.timeline,
        resources: {
          resourceFilter: { search: 'x'.repeat(MAX_RESOURCE_FILTER_QUERY_LENGTH + 1) },
        },
      })
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
    expect(DeepLinkStateV1Schema.validate({ zoomRange: { start: 20, end: 10 } })).toBe(false);
    expect(
      DeepLinkStateV1Schema.validate({
        zoomRange: { start: 1, end: 2 },
        expandedResourceIds: [''],
      })
    ).toBe(false);
  });

  it('derives enum vocabularies from shared constants', () => {
    const base = { route: state.route, timeline: state.timeline };
    for (const palette of Object.keys(CONTINUOUS_PALETTES)) {
      expect(
        DeepLinkStateV3Schema.validate({
          ...base,
          dag: { nodeColorPalette: palette, edgeColorPalette: palette },
        })
      ).toBe(true);
    }
    for (const nodeLabelField of Object.values(NODE_LABEL_FIELD)) {
      expect(DeepLinkStateV3Schema.validate({ ...base, dag: { nodeLabelField } })).toBe(true);
    }
    for (const layoutDirection of Object.values(DAG_LAYOUT_DIRECTION)) {
      expect(DeepLinkStateV3Schema.validate({ ...base, dag: { layoutDirection } })).toBe(true);
    }
    for (const scaleType of Object.values(SCALE_TYPE)) {
      expect(DeepLinkStateV3Schema.validate({ ...base, dag: { scaleType } })).toBe(true);
    }
    expect(OperatorGroupSchema.options).toEqual([...OPERATOR_TABLE_INDEX_ORDER]);
  });
});

it('round trips structural pipe inspection independently of operator selection and nested metric paths', () => {
  const shared: DeepLinkStateV3 = {
    ...state,
    selection: {
      planId: 'p',
      operatorNodeIds: ['old'],
      pipes: [
        { sourcePortId: 'out-1', targetPortId: 'in-1' },
        { sourcePortId: 'out-2', targetPortId: 'in-2' },
      ],
    },
    dag: { edgeWidthField: '[["Volume",0],["bytes",0]]' },
  };
  const encoded = encodeDeepLinkState(shared);
  expect(encoded.ok).toBe(true);
  if (!encoded.ok) {
    return;
  }
  const decoded = decodeDeepLinkState(encoded.value);
  expect(decoded.ok).toBe(true);
  if (!decoded.ok) {
    return;
  }
  const restored = DeepLinkStateV3Schema.parse(decoded.value.data);
  expect(restored.selection).toEqual(shared.selection);
  expect(restored.dag?.edgeWidthField).toBe(shared.dag?.edgeWidthField);
});

describe('deep-link pinned rows', () => {
  const withPins = (pinnedRowIds: string[]): DeepLinkStateV3 => ({
    route: state.route,
    timeline: state.timeline,
    resources: { pinnedRowIds },
  });

  it('round-trips pinned row ids in the order they were arranged', () => {
    const encoded = encodeDeepLinkState(withPins([RESOURCE_B_ID, RESOURCE_A_ID]));
    expect(encoded.ok).toBe(true);
    if (!encoded.ok) {
      return;
    }

    expect(decodeDeepLinkState(encoded.value)).toEqual({
      ok: true,
      value: { version: 'v3', data: withPins([RESOURCE_B_ID, RESOURCE_A_ID]) },
    });
  });

  it('drops repeated pinned row ids, keeping the first of each', () => {
    const parsed = DeepLinkStateV3Schema.parse(
      withPins([RESOURCE_B_ID, RESOURCE_A_ID, RESOURCE_B_ID])
    );

    expect(parsed.resources?.pinnedRowIds).toEqual([RESOURCE_B_ID, RESOURCE_A_ID]);
  });

  it('accepts the ids of Entities and Operators rows', () => {
    const laneId = `__long_entities__${RESOURCE_A_ID}`;
    const operatorsId = `__operator_timeline__${RESOURCE_B_ID}`;

    const parsed = DeepLinkStateV3Schema.parse(withPins([operatorsId, laneId]));

    expect(parsed.resources?.pinnedRowIds).toEqual([operatorsId, laneId]);
  });

  it('allows the maximum number of pins and rejects more', () => {
    const ids = (count: number) => Array.from({ length: count }, (_, i) => `row-${i}`);

    expect(DeepLinkStateV3Schema.validate(withPins(ids(MAX_PINNED_ROWS)))).toBe(true);
    expect(encodeDeepLinkState(withPins(ids(MAX_PINNED_ROWS + 1)))).toEqual({
      ok: false,
      code: 'invalid-state',
      message: `Shared links support at most ${MAX_PINNED_ROWS} pinned rows.`,
      messages: [`Shared links support at most ${MAX_PINNED_ROWS} pinned rows.`],
    });
  });

  it('leaves links without pins unchanged', () => {
    const parsed = DeepLinkStateV3Schema.parse({ route: state.route, timeline: state.timeline });

    expect(parsed.resources).toBeUndefined();
  });
});
