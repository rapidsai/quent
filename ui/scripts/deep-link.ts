// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { readFile } from 'node:fs/promises';
import { parseArgs } from 'node:util';
import {
  buildViewStateUrl,
  decodeViewState,
  VIEW_STATE_SEARCH_KEY,
} from '../src/features/view-state/viewState.codec';
import {
  ViewStateV3Schema,
  type ViewStateV3,
  type ViewStateTab,
} from '../src/features/view-state/viewState.schema';
import { mergeResourceFilter } from '../src/features/view-state/viewState.cli';

const usage = `Usage:
  pnpm deep-link create --engine ID --query ID --tab timeline --start S --end S [--resource-search TEXT] [--resource-types TYPES] [--fsm-types TYPES] [--show-others] [--base URL]
  pnpm deep-link create --engine ID --query ID --tab TAB --state FILE [--resource-search TEXT] [--resource-types TYPES] [--fsm-types TYPES] [--show-others] [--base URL]
  pnpm deep-link decode URL`;

function fail(message: string): never {
  console.error(message);
  console.error(usage);
  process.exit(1);
}

function parseList(value: string | boolean | undefined): string[] | undefined {
  if (typeof value !== 'string') {
    return undefined;
  }
  const values = value
    .split(',')
    .map(item => item.trim())
    .filter(Boolean);
  return values.length > 0 ? values : undefined;
}

async function readState(
  values: Record<string, string | boolean | undefined>,
  route: ViewStateV3['route']
) {
  let input: unknown;
  if (typeof values.state === 'string') {
    input = JSON.parse(await readFile(values.state, 'utf8')) as unknown;
  } else {
    const start = Number(values.start);
    const end = Number(values.end);
    input = { timeline: { zoomRange: { start, end } } };
  }

  if (!input || typeof input !== 'object' || Array.isArray(input)) {
    fail('State must be a JSON object.');
  }
  const raw = input as Record<string, unknown>;
  const candidate =
    'zoomRange' in raw
      ? {
          route,
          timeline: { zoomRange: raw.zoomRange },
          ...('expandedResourceIds' in raw
            ? { resources: { expandedRowIds: raw.expandedResourceIds } }
            : {}),
        }
      : { ...raw, route };
  const resourceTypes = parseList(values['resource-types']);
  const fsmTypes = parseList(values['fsm-types']);
  const resourceFilter = {
    ...(typeof values['resource-search'] === 'string' ? { search: values['resource-search'] } : {}),
    ...(resourceTypes ? { resourceTypes } : {}),
    ...(fsmTypes ? { fsmTypes } : {}),
    ...(values['show-others'] === true ? { showOthers: true } : {}),
  };
  const candidateWithFilter = mergeResourceFilter(candidate, resourceFilter);
  const parsed = ViewStateV3Schema.safeParse(candidateWithFilter);
  if (!parsed.success) {
    fail(`Invalid state: ${parsed.error.message}`);
  }
  return parsed.data;
}

function parseTab(tab: string): ViewStateTab {
  if (tab !== 'timeline' && tab !== 'operators' && tab !== 'entities') {
    fail('The --tab option must be "timeline", "operators", or "entities".');
  }
  return tab;
}

function buildRoute(engineId: string, queryId: string, tab: ViewStateTab): string {
  return `/profile/engine/${encodeURIComponent(engineId)}/query/${encodeURIComponent(queryId)}/${tab}`;
}

async function createLink(values: Record<string, string | boolean | undefined>) {
  if (typeof values.engine !== 'string') {
    fail('Missing --engine.');
  }
  if (typeof values.query !== 'string') {
    fail('Missing --query.');
  }
  if (typeof values.tab !== 'string') {
    fail('Missing --tab.');
  }
  if (
    typeof values.state !== 'string' &&
    (values.start === undefined || values.end === undefined)
  ) {
    fail('Provide --state FILE or both --start and --end.');
  }

  const tab = parseTab(values.tab);
  const stateRoute = { engineId: values.engine, queryId: values.query, tab };
  const state = await readState(values, stateRoute);
  const route = buildRoute(values.engine, values.query, tab);
  const currentUrl =
    typeof values.base === 'string' ? new URL(route, values.base).toString() : route;
  const result = buildViewStateUrl(currentUrl, state);
  if (!result.ok) {
    fail(result.message);
  }
  process.stdout.write(`${result.value}\n`);
}

function decodeLink(input: string | undefined) {
  if (!input) {
    fail('Missing URL to decode.');
  }
  const url = new URL(input, 'http://deep-link.invalid');
  const encoded = url.searchParams.get(VIEW_STATE_SEARCH_KEY);
  if (!encoded) {
    fail(`The URL has no "${VIEW_STATE_SEARCH_KEY}" parameter.`);
  }

  const result = decodeViewState(encoded);
  if (!result.ok) {
    fail(result.message);
  }
  process.stdout.write(`${JSON.stringify(result.value, null, 2)}\n`);
}

async function main() {
  const { positionals, values } = parseArgs({
    allowPositionals: true,
    options: {
      base: { type: 'string' },
      end: { type: 'string' },
      engine: { type: 'string' },
      'fsm-types': { type: 'string' },
      query: { type: 'string' },
      'resource-search': { type: 'string' },
      'resource-types': { type: 'string' },
      'show-others': { type: 'boolean' },
      start: { type: 'string' },
      state: { type: 'string' },
      tab: { type: 'string' },
    },
  });

  switch (positionals[0]) {
    case 'create':
      await createLink(values);
      break;
    case 'decode':
      decodeLink(positionals[1]);
      break;
    default:
      fail('Expected "create" or "decode".');
  }
}

await main();
