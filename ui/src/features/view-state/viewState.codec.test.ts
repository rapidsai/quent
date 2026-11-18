// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { gzipSync, strToU8 } from 'fflate';
import {
  buildViewStateUrl,
  buildViewStateUrlFromEncodedState,
  CURRENT_VIEW_STATE_VERSION,
  decodeViewState,
  encodeViewState,
  MAX_VIEW_STATE_URL_LENGTH,
  SUPPORTED_VIEW_STATE_VERSIONS,
} from './viewState.codec';
import {
  ViewStateV1Schema,
  ViewStateV2Schema,
  ViewStateV3Schema,
  MAX_EXPANDED_RESOURCE_IDS,
  MAX_PINNED_ROW_IDS,
  MAX_SELECTED_NODE_IDS,
  OperatorGroupSchema,
  type ViewStateV3,
  validateViewStateSearch,
} from './viewState.schema';
import {
  statisticFieldId,
  CONTINUOUS_PALETTES,
  DAG_LAYOUT_DIRECTION,
  SCALE_TYPE,
  NODE_LABEL_FIELD,
} from '@quent/utils';
import { OPERATOR_TABLE_INDEX_ORDER } from '@/components/operator-table/types';
import { MAX_RESOURCE_FILTER_QUERY_LENGTH } from '@/features/resource-filter/resourceFilter';
import {
  DEEP_LINK_COMPATIBILITY_PAYLOAD,
  DEEP_LINK_COMPATIBILITY_STATE,
} from '../../../../experimental/vibe/quent-cli/src/deepLink.compatibility';

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

const state: ViewStateV3 = {
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
    longEntityDensity: 2,
  },
  selection: {
    planId: 'plan-a',
    operatorNodeIds: ['operator-a', 'operator-b'],
  },
  resources: {
    expandedRowIds: ['resource-a', 'resource-b'],
    pinnedRowsCollapsed: true,
    pinnedRowsPanelSize: 240,
    nvtxSelection: {
      domainId: 'nvtx-domain-a',
      categories: [{ domainId: 'nvtx-domain-a', categoryId: 'category-a' }],
    },
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

describe('view-state codec', () => {
  it('round-trips deterministically', () => {
    const first = encodeViewState(state);
    const second = encodeViewState(state);

    expect(first).toEqual(second);
    expect(first.ok).toBe(true);
    if (!first.ok) {
      return;
    }
    expect(decodeViewState(first.value)).toEqual({
      ok: true,
      value: { version: 'v3', data: state },
    });
  });

  it('round-trips explicit statistic paths longer than the generic name limit', () => {
    const fieldId = statisticFieldId([
      ['Volume '.repeat(100), 0],
      ['bytes '.repeat(100), 2],
    ]);
    const nestedState: ViewStateV3 = {
      ...state,
      dag: {
        ...state.dag,
        nodeColorField: fieldId,
        edgeWidthField: fieldId,
        edgeColorField: fieldId,
      },
    };
    const encoded = encodeViewState(nestedState);
    expect(encoded.ok).toBe(true);
    if (!encoded.ok) {
      throw new Error('Failed to encode nested statistic selections');
    }
    expect(decodeViewState(encoded.value)).toEqual({
      ok: true,
      value: { version: 'v3', data: nestedState },
    });
  });

  it('keeps the current version in the supported schema registry', () => {
    expect(SUPPORTED_VIEW_STATE_VERSIONS).toContain(CURRENT_VIEW_STATE_VERSION);
    expect(new Set(SUPPORTED_VIEW_STATE_VERSIONS).size).toBe(SUPPORTED_VIEW_STATE_VERSIONS.length);
  });

  it('rejects unsupported versions and malformed payloads', () => {
    expect(decodeViewState('v999.abc')).toMatchObject({
      ok: false,
      code: 'unsupported-version',
    });
    expect(decodeViewState('v2.not-gzip')).toMatchObject({
      ok: false,
      code: 'invalid-encoding',
    });
  });

  it('decodes legacy v1 links', () => {
    const encoded = 'v1.H4sIAAAAAAACA6tWqsrPzw1KzEtPVbKqViouSSwqUbIy0FFKzUsB0nomBua1tQAidcVYJQAAAA';

    expect(decodeViewState(encoded)).toEqual({
      ok: true,
      value: { version: 'v1', data: { zoomRange: { start: 0, end: 0.407 } } },
    });
  });

  it('decodes legacy v2 links', () => {
    const legacyState = ViewStateV2Schema.parse({
      route: { engineId: 'engine-a', queryId: 'query-a', tab: 'operators' },
      timeline: { zoomRange: { start: 12.5, end: 48.75 } },
      selection: { operatorNodeIds: ['operator-a'] },
    });
    const encoded = encodeRawState('v2', legacyState);

    expect(decodeViewState(encoded)).toEqual({
      ok: true,
      value: { version: 'v2', data: legacyState },
    });
  });

  it('builds links within the absolute URL budget', () => {
    const result = buildViewStateUrl(
      'https://quent.example.test/profile/engine/e/query/q/timeline',
      state
    );

    expect(result.ok).toBe(true);
    if (!result.ok) {
      return;
    }
    expect(result.value.length).toBeLessThanOrEqual(MAX_VIEW_STATE_URL_LENGTH);
    expect(new URL(result.value).searchParams.get('s')).toMatch(
      new RegExp(`^${CURRENT_VIEW_STATE_VERSION}\\.`)
    );
  });

  it('builds a link from state encoded by the boundary', () => {
    const encoded = encodeViewState(state);
    expect(encoded.ok).toBe(true);
    if (!encoded.ok) {
      return;
    }

    const result = buildViewStateUrlFromEncodedState(
      '/profile/engine/e/query/q/entities',
      encoded.value
    );
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(new URL(result.value, 'https://quent.test').searchParams.get('s')).toBe(encoded.value);
    }
  });

  it('decodes links produced by the experimental CLI', () => {
    expect(decodeViewState(DEEP_LINK_COMPATIBILITY_PAYLOAD)).toEqual({
      ok: true,
      value: { version: 'v3', data: DEEP_LINK_COMPATIBILITY_STATE },
    });
  });

  it('rejects an absolute URL whose origin and path exhaust the budget', () => {
    const result = buildViewStateUrl(
      `https://quent.example.test/${'x'.repeat(MAX_VIEW_STATE_URL_LENGTH)}`,
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
      ViewStateV3Schema.safeParse({
        route: state.route,
        selection: { operatorNodeIds: atLimit },
      }).success
    ).toBe(true);

    expect(
      encodeViewState({
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
      encodeViewState({
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

    expect(encodeViewState(invalidState)).toEqual(expected);
    expect(decodeViewState(encodeRawState('v3', invalidState))).toEqual(expected);
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

    expect(encodeViewState(invalidState)).toMatchObject({
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

describe('view-state search validation', () => {
  it('strips unknown keys and rejects non-string state', () => {
    expect(validateViewStateSearch({ s: 'v1.abc', extra: true })).toEqual({ s: 'v1.abc' });
    expect(validateViewStateSearch({ s: 42 })).toEqual({});
  });
});

describe('view state validation', () => {
  it('validates and canonicalizes comprehensive v3 state', () => {
    expect(ViewStateV3Schema.parse({ ...state, futureField: true })).toEqual(state);
    expect(
      ViewStateV3Schema.parse({
        route: state.route,
        timeline: state.timeline,
        resources: {
          expandedRowIds: [RESOURCE_B_ID, NVTX_SECTION_ID, RESOURCE_A_ID, RESOURCE_B_ID],
        },
      }).resources?.expandedRowIds
    ).toEqual([RESOURCE_A_ID, RESOURCE_B_ID, NVTX_SECTION_ID]);
    expect(
      ViewStateV3Schema.validate({
        route: state.route,
        timeline: { zoomRange: { start: 20, end: 10 } },
      })
    ).toBe(false);
    expect(
      ViewStateV3Schema.validate({
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
      ViewStateV3Schema.validate({
        route: state.route,
        timeline: state.timeline,
        resources: {
          resourceFilter: { search: 'x'.repeat(MAX_RESOURCE_FILTER_QUERY_LENGTH + 1) },
        },
      })
    ).toBe(false);
    expect(
      ViewStateV3Schema.safeParse({
        route: state.route,
        entities: { window: { start: 20, end: 10 } },
      }).success
    ).toBe(false);
  });

  it('keeps the v2 schema unchanged', () => {
    expect(
      ViewStateV2Schema.safeParse({
        route: { engineId: 'engine-a', queryId: 'query-a', tab: 'entities' },
        timeline: state.timeline,
      }).success
    ).toBe(false);
  });

  it('keeps the legacy v1 schema available for old links', () => {
    expect(
      ViewStateV1Schema.parse({
        zoomRange: { start: 1, end: 2 },
        expandedResourceIds: ['b', 'a'],
      })
    ).toEqual({
      zoomRange: { start: 1, end: 2 },
      expandedResourceIds: ['a', 'b'],
    });
    expect(
      ViewStateV1Schema.parse({
        zoomRange: { start: 1, end: 2 },
        expandedResourceIds: [RESOURCE_B_ID, NVTX_SECTION_ID, RESOURCE_A_ID, RESOURCE_B_ID],
      }).expandedResourceIds
    ).toEqual([RESOURCE_A_ID, RESOURCE_B_ID, NVTX_SECTION_ID]);
    expect(ViewStateV1Schema.validate({ zoomRange: { start: 20, end: 10 } })).toBe(false);
    expect(
      ViewStateV1Schema.validate({
        zoomRange: { start: 1, end: 2 },
        expandedResourceIds: [''],
      })
    ).toBe(false);
  });

  it('derives enum vocabularies from shared constants', () => {
    const base = { route: state.route, timeline: state.timeline };
    for (const palette of Object.keys(CONTINUOUS_PALETTES)) {
      expect(
        ViewStateV3Schema.validate({
          ...base,
          dag: { nodeColorPalette: palette, edgeColorPalette: palette },
        })
      ).toBe(true);
    }
    for (const nodeLabelField of Object.values(NODE_LABEL_FIELD)) {
      expect(ViewStateV3Schema.validate({ ...base, dag: { nodeLabelField } })).toBe(true);
    }
    for (const layoutDirection of Object.values(DAG_LAYOUT_DIRECTION)) {
      expect(ViewStateV3Schema.validate({ ...base, dag: { layoutDirection } })).toBe(true);
    }
    for (const scaleType of Object.values(SCALE_TYPE)) {
      expect(ViewStateV3Schema.validate({ ...base, dag: { scaleType } })).toBe(true);
    }
    expect(OperatorGroupSchema.options).toEqual([...OPERATOR_TABLE_INDEX_ORDER]);
  });
});

it('round trips pipe inspection independently of operator selection and nested metric paths', () => {
  const shared: ViewStateV3 = {
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
  const encoded = encodeViewState(shared);
  expect(encoded.ok).toBe(true);
  if (!encoded.ok) {
    return;
  }
  const decoded = decodeViewState(encoded.value);
  expect(decoded.ok).toBe(true);
  if (!decoded.ok) {
    return;
  }
  const restored = ViewStateV3Schema.parse(decoded.value.data);
  expect(restored.selection).toEqual(shared.selection);
  expect(restored.dag?.edgeWidthField).toBe(shared.dag?.edgeWidthField);
});

describe('deep-link collapsed default rows', () => {
  it('round-trips collapsed row ids', () => {
    const shared: ViewStateV3 = {
      route: state.route,
      timeline: state.timeline,
      resources: { collapsedRowIds: [RESOURCE_A_ID] },
    };
    const encoded = encodeViewState(shared);
    expect(encoded.ok).toBe(true);
    if (!encoded.ok) {
      return;
    }
    expect(decodeViewState(encoded.value)).toEqual({
      ok: true,
      value: { version: 'v3', data: shared },
    });
  });
});

describe('deep-link nested chart rows', () => {
  it('round-trips the nested chart rows setting', () => {
    const shared: ViewStateV3 = {
      route: state.route,
      timeline: state.timeline,
      resources: { showNestedChartRows: true },
    };
    const encoded = encodeViewState(shared);
    expect(encoded.ok).toBe(true);
    if (!encoded.ok) {
      return;
    }

    expect(decodeViewState(encoded.value)).toEqual({
      ok: true,
      value: { version: 'v3', data: shared },
    });
  });
});

it('normalizes legacy detail IDs into filters and preserves nested metric paths', () => {
  const shared = {
    ...state,
    selection: {
      planId: 'p',
      operatorNodeIds: ['old'],
      operatorDetailsNodeIds: ['details-a', 'details-b'],
      pipes: [
        { sourcePortId: 'out-1', targetPortId: 'in-1' },
        { sourcePortId: 'out-2', targetPortId: 'in-2' },
      ],
    },
    dag: { edgeWidthField: '[["Volume",0],["bytes",0]]' },
  };
  const decoded = decodeViewState(encodeRawState('v3', shared));
  expect(decoded.ok).toBe(true);
  if (!decoded.ok) {
    return;
  }
  expect(decoded.value.version).toBe('v3');
  if (decoded.value.version !== 'v3') {
    return;
  }
  const restored = ViewStateV3Schema.parse(decoded.value.data);
  expect(restored.selection).toEqual({
    planId: 'p',
    operatorNodeIds: ['details-a', 'details-b', 'old'],
    pipes: shared.selection.pipes,
  });
  expect(restored.dag?.edgeWidthField).toBe(shared.dag.edgeWidthField);
});

describe('view-state pinned rows', () => {
  const withPins = (pinnedRowIds: string[]): ViewStateV3 => ({
    route: state.route,
    timeline: state.timeline,
    resources: { pinnedRowIds },
  });

  it('round-trips pinned row ids in the order they were arranged', () => {
    const encoded = encodeViewState(withPins([RESOURCE_B_ID, RESOURCE_A_ID]));
    expect(encoded.ok).toBe(true);
    if (!encoded.ok) {
      return;
    }

    expect(decodeViewState(encoded.value)).toEqual({
      ok: true,
      value: { version: 'v3', data: withPins([RESOURCE_B_ID, RESOURCE_A_ID]) },
    });
  });

  it('drops repeated pinned row ids, keeping the first of each', () => {
    const parsed = ViewStateV3Schema.parse(withPins([RESOURCE_B_ID, RESOURCE_A_ID, RESOURCE_B_ID]));

    expect(parsed.resources?.pinnedRowIds).toEqual([RESOURCE_B_ID, RESOURCE_A_ID]);
  });

  it('accepts the ids of Entities and Operators rows', () => {
    const laneId = `__long_entities__${RESOURCE_A_ID}`;
    const operatorsId = `__operator_timeline__${RESOURCE_B_ID}`;

    const parsed = ViewStateV3Schema.parse(withPins([operatorsId, laneId]));

    expect(parsed.resources?.pinnedRowIds).toEqual([operatorsId, laneId]);
  });

  it('allows the maximum number of pins and rejects more', () => {
    const ids = (count: number) => Array.from({ length: count }, (_, i) => `row-${i}`);

    expect(ViewStateV3Schema.validate(withPins(ids(MAX_PINNED_ROW_IDS)))).toBe(true);
    expect(encodeViewState(withPins(ids(MAX_PINNED_ROW_IDS + 1)))).toEqual({
      ok: false,
      code: 'invalid-state',
      message: `Shared links support at most ${MAX_PINNED_ROW_IDS} pinned rows.`,
      messages: [`Shared links support at most ${MAX_PINNED_ROW_IDS} pinned rows.`],
    });
  });

  it('leaves links without pins unchanged', () => {
    const parsed = ViewStateV3Schema.parse({ route: state.route, timeline: state.timeline });

    expect(parsed.resources).toBeUndefined();
  });
});
