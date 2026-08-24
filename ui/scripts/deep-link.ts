// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { readFile } from 'node:fs/promises';
import { parseArgs } from 'node:util';
import {
  buildDeepLinkUrl,
  decodeDeepLinkState,
  DEEP_LINK_SEARCH_KEY,
} from '../src/features/deep-link/deepLink.codec';
import {
  DeepLinkStateV1Schema,
  type DeepLinkStateV1,
} from '../src/features/deep-link/deepLink.schema';

const usage = `Usage:
  pnpm deep-link create --engine ID --query ID --tab timeline --start S --end S [--base URL]
  pnpm deep-link create --engine ID --query ID --tab timeline --state FILE [--base URL]
  pnpm deep-link decode URL`;

function fail(message: string): never {
  console.error(message);
  console.error(usage);
  process.exit(1);
}

async function readState(values: Record<string, string | boolean | undefined>) {
  let input: unknown;
  if (typeof values.state === 'string') {
    input = JSON.parse(await readFile(values.state, 'utf8')) as unknown;
  } else {
    const start = Number(values.start);
    const end = Number(values.end);
    input = { zoomRange: { start, end } };
  }

  const parsed = DeepLinkStateV1Schema.safeParse(input);
  if (!parsed.success) fail(`Invalid state: ${parsed.error.message}`);
  return parsed.data;
}

function buildRoute(engineId: string, queryId: string, tab: string): string {
  if (tab !== 'timeline' && tab !== 'operators') {
    fail('The --tab option must be "timeline" or "operators".');
  }
  return `/profile/engine/${encodeURIComponent(engineId)}/query/${encodeURIComponent(queryId)}/${tab}`;
}

async function createLink(values: Record<string, string | boolean | undefined>) {
  if (typeof values.engine !== 'string') fail('Missing --engine.');
  if (typeof values.query !== 'string') fail('Missing --query.');
  if (typeof values.tab !== 'string') fail('Missing --tab.');
  if (
    typeof values.state !== 'string' &&
    (values.start === undefined || values.end === undefined)
  ) {
    fail('Provide --state FILE or both --start and --end.');
  }

  const state: DeepLinkStateV1 = await readState(values);
  const route = buildRoute(values.engine, values.query, values.tab);
  const currentUrl =
    typeof values.base === 'string' ? new URL(route, values.base).toString() : route;
  const result = buildDeepLinkUrl(currentUrl, state);
  if (!result.ok) fail(result.message);
  process.stdout.write(`${result.value}\n`);
}

function decodeLink(input: string | undefined) {
  if (!input) fail('Missing URL to decode.');
  const url = new URL(input, 'http://deep-link.invalid');
  const encoded = url.searchParams.get(DEEP_LINK_SEARCH_KEY);
  if (!encoded) fail(`The URL has no "${DEEP_LINK_SEARCH_KEY}" parameter.`);

  const result = decodeDeepLinkState(encoded);
  if (!result.ok) fail(result.message);
  process.stdout.write(`${JSON.stringify(result.value, null, 2)}\n`);
}

async function main() {
  const { positionals, values } = parseArgs({
    allowPositionals: true,
    options: {
      base: { type: 'string' },
      end: { type: 'string' },
      engine: { type: 'string' },
      query: { type: 'string' },
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
