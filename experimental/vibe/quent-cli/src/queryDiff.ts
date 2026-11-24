// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { parseArgs } from 'node:util';
import { commonQueryMetrics, diffQueryBundles } from '@quent/query-analysis';
import { type EntityRef, type QueryBundle } from '@quent/utils';
import {
  createTerminalSelector,
  discoverQueryTreeChoices,
  selectEngine,
  selectQuery,
  type DiscoveryApi,
  type QuerySelection,
  type QueryTreeChoice,
  type SelectFromList,
  type SelectFromQueryTree,
} from './askSelection';
import type { AskCommand } from './askCommand.types';
import { selectFromQueryTreeWithInk, selectManyWithInk } from './inkSelectorRunner';
import { machineResult, serializeMachineOutput } from './machineOutput';
import { launchQuentOpenDbRuns, stopQuentOpenDbRuns } from './quentOpenDb';
import { formatQueryDiff } from './queryDiffFormat';
import {
  createHttpQuerySourceApi,
  inputDatabaseRunIds,
  querySourceUsage,
  resolveDatabaseRunIds,
  withQuerySources,
  type InputText,
  type QuerySourceRuntime,
} from './querySources';

const usage = `Usage:
  pnpm ask query-diff [options]

Options:
  --engine ID             Default engine for the baseline and candidates
  --baseline-engine ID    Baseline engine (overrides --engine)
  --baseline-query ID     Baseline query (select from API results when omitted)
  --candidate-query ID    Candidate query; repeat for multiple on one engine
  --candidate-engine ID   Candidate engine; one for all queries or one per query
  --candidate ENGINE:QUERY
                          Qualified candidate; repeat for cross-engine comparison
  --metric NAME           Metric to compare; repeat or pass "all"
  --[no-]combined-table   Combine operator rows into one table (default: true)
${querySourceUsage}
  --baseline-source ID    "local" or a registered database run
  --candidate-source ID   Candidate source; one for all or one per candidate
                          Maps --candidate entries before --candidate-query entries
  --json                  Emit versioned JSON output (selection may still be interactive)`;

type QueryDiffCliValue = string | string[] | boolean | undefined;
type QueryDiffCliValues = Record<string, QueryDiffCliValue>;

function optionalId(value: QueryDiffCliValue): string | undefined {
  return typeof value === 'string' && value.trim() !== '' ? value.trim() : undefined;
}

function ids(value: QueryDiffCliValue): string[] {
  if (Array.isArray(value)) {
    return value.map(item => item.trim()).filter(Boolean);
  }
  const id = optionalId(value);
  return id ? [id] : [];
}

export interface QueryDiffSource {
  id: string;
  label: string;
  api: DiscoveryApi;
}

export interface SourcedQuerySelection extends QuerySelection {
  sourceId: string;
  sourceLabel: string;
}

export interface SourcedQueryDiffSelections {
  baseline: SourcedQuerySelection;
  candidates: SourcedQuerySelection[];
}

type SelectMetrics = (
  prompt: string,
  choices: readonly { value: string; label: string }[],
  allValue?: string,
  signal?: AbortSignal
) => Promise<string[]>;

const ALL_METRICS = '\0all-metrics';

export interface QueryDiffInteraction {
  select: SelectFromList;
  selectTree: SelectFromQueryTree;
  selectMetrics: SelectMetrics;
  inputText: InputText;
}

export interface QueryDiffRuntime {
  createDiscoveryApi(baseUrl: string, signal?: AbortSignal): DiscoveryApi;
  launchDatabaseRuns: QuerySourceRuntime['launch'];
  stopDatabaseRuns: QuerySourceRuntime['stop'];
}

export async function resolveQueryDiffMetrics(
  values: QueryDiffCliValues,
  bundles: readonly QueryBundle<EntityRef>[],
  selectMetrics: SelectMetrics = selectManyWithInk,
  signal?: AbortSignal
): Promise<string[]> {
  const commonMetrics = commonQueryMetrics(bundles);
  if (commonMetrics.length === 0) {
    throw new Error('The selected queries have no numeric metrics in common.');
  }
  let requested = ids(values.metric);
  if (requested.length === 0) {
    const choices = [
      { value: ALL_METRICS, label: `All metrics (${commonMetrics.length})` },
      ...commonMetrics.map(metric => ({ value: metric, label: metric })),
    ];
    requested = await (signal
      ? selectMetrics('Select metrics to compare', choices, ALL_METRICS, signal)
      : selectMetrics('Select metrics to compare', choices, ALL_METRICS));
  }
  if (
    requested.includes(ALL_METRICS) ||
    requested.some(metric => metric.toLocaleLowerCase() === 'all')
  ) {
    return commonMetrics;
  }
  const invalid = requested.filter(metric => !commonMetrics.includes(metric));
  if (invalid.length > 0) {
    throw new Error(
      `Metrics not available in every selected query: ${invalid.join(', ')}. Available metrics: ${commonMetrics.join(', ')}.`
    );
  }
  return [...new Set(requested)];
}

function parseQualifiedCandidate(value: string): QuerySelection {
  const separator = value.indexOf(':');
  const engineId = value.slice(0, separator).trim();
  const queryId = value.slice(separator + 1).trim();
  if (separator <= 0 || !engineId || !queryId) {
    throw new Error(`Invalid candidate "${value}"; expected ENGINE:QUERY.`);
  }
  return { engineId, queryId };
}

function candidateEngineIds(
  queryIds: string[],
  engines: string[],
  commonEngineId: string | undefined
): string[] | null {
  if (engines.length === 1) {
    return queryIds.map(() => engines[0]!);
  }
  if (engines.length === queryIds.length) {
    return engines;
  }
  if (engines.length === 0 && commonEngineId) {
    return queryIds.map(() => commonEngineId);
  }
  if (engines.length === 0) {
    return null;
  }
  throw new Error(
    'Pass one --candidate-engine for all candidate queries or one --candidate-engine per query.'
  );
}

function findSource(sources: readonly QueryDiffSource[], id: string): QueryDiffSource {
  const source = sources.find(candidate => candidate.id === id);
  if (!source) {
    throw new Error(
      `Unknown query source "${id}". Available sources: ${sources.map(source => source.id).join(', ')}.`
    );
  }
  return source;
}

async function selectSource(
  sources: readonly QueryDiffSource[],
  select: SelectFromList,
  prompt: string,
  signal?: AbortSignal
): Promise<QueryDiffSource> {
  if (sources.length === 1) {
    return sources[0]!;
  }
  const choices = sources.map(source => ({ value: source.id, label: source.label }));
  const id = await (signal ? select(prompt, choices, signal) : select(prompt, choices));
  return findSource(sources, id);
}

interface SourcedQueryDiscovery {
  choices: Array<QueryTreeChoice & { sourceId: string; sourceLabel: string }>;
  failures: ReadonlyMap<string, unknown>;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

async function discoverSourcedQueryTreeChoices(
  sources: readonly QueryDiffSource[]
): Promise<SourcedQueryDiscovery> {
  const includeSourceLabel = sources.length !== 1 || sources[0]?.id !== 'local';
  const results = await Promise.allSettled(
    sources.map(async source => ({
      source,
      choices: (await discoverQueryTreeChoices(source.api)).map(choice => ({
        ...choice,
        sourceId: source.id,
        sourceLabel: source.label,
        value: `${source.id}\0${choice.value}`,
        engineLabel: includeSourceLabel
          ? `${source.label} · ${choice.engineLabel}`
          : choice.engineLabel,
      })),
    }))
  );
  const choices = results.flatMap(result =>
    result.status === 'fulfilled' ? result.value.choices : []
  );
  const failures = new Map<string, unknown>();
  results.forEach((result, index) => {
    if (result.status === 'rejected') {
      const source = sources[index]!;
      failures.set(source.id, result.reason);
      process.stderr.write(
        `Warning: unable to discover queries from source "${source.label}": ${errorMessage(result.reason)}\n`
      );
    }
  });
  if (choices.length === 0) {
    const firstFailure = results.find(
      (result): result is PromiseRejectedResult => result.status === 'rejected'
    );
    if (firstFailure) {
      throw firstFailure.reason;
    }
  }
  return { choices, failures };
}

function selectedFromTree(
  selected: readonly string[],
  choices: readonly (QueryTreeChoice & {
    sourceId: string;
    sourceLabel: string;
  })[]
): SourcedQuerySelection[] {
  const byValue = new Map(choices.map(choice => [choice.value, choice]));
  return selected.map(value => {
    const choice = byValue.get(value);
    if (!choice) {
      throw new Error(`Selected query "${value}" is unavailable.`);
    }
    return {
      sourceId: choice.sourceId,
      sourceLabel: choice.sourceLabel,
      engineId: choice.engineId,
      queryId: choice.queryId,
    };
  });
}

function explicitCandidateSources(
  count: number,
  requested: string[],
  sources: readonly QueryDiffSource[]
): QueryDiffSource[] | null {
  if (requested.length === 1) {
    return Array.from({ length: count }, () => findSource(sources, requested[0]!));
  }
  if (requested.length === count) {
    return requested.map(id => findSource(sources, id));
  }
  if (requested.length === 0 && sources.length === 1) {
    return Array.from({ length: count }, () => sources[0]!);
  }
  if (requested.length === 0) {
    return null;
  }
  throw new Error(
    'Pass one --candidate-source for all candidates or one --candidate-source per candidate.'
  );
}

function uniqueSourcedCandidates(
  baseline: SourcedQuerySelection,
  candidates: SourcedQuerySelection[]
): SourcedQuerySelection[] {
  const baselineKey = `${baseline.sourceId}\0${baseline.engineId}\0${baseline.queryId}`;
  const seen = new Set<string>();
  return candidates.filter(candidate => {
    const key = `${candidate.sourceId}\0${candidate.engineId}\0${candidate.queryId}`;
    if (key === baselineKey) {
      throw new Error('The baseline query cannot also be a candidate.');
    }
    if (seen.has(key)) {
      return false;
    }
    seen.add(key);
    return true;
  });
}

export async function resolveSourcedQueryDiffSelections(
  values: QueryDiffCliValues,
  sources: readonly QueryDiffSource[],
  select: SelectFromList,
  selectTree: SelectFromQueryTree = selectFromQueryTreeWithInk,
  signal?: AbortSignal
): Promise<SourcedQueryDiffSelections> {
  if (sources.length === 0) {
    throw new Error('No query sources are available.');
  }
  const commonEngineId = optionalId(values.engine);
  const baselineSourceId = optionalId(values['baseline-source']);
  const explicitBaselineEngineId = optionalId(values['baseline-engine']) ?? commonEngineId;
  const explicitBaselineQueryId = optionalId(values['baseline-query']);
  let combinedDiscovery: SourcedQueryDiscovery | undefined;
  const getCombinedDiscovery = async () => {
    combinedDiscovery ??= await discoverSourcedQueryTreeChoices(sources);
    return combinedDiscovery;
  };

  let baseline: SourcedQuerySelection;
  if (!explicitBaselineEngineId && !explicitBaselineQueryId) {
    if (baselineSourceId) {
      findSource(sources, baselineSourceId);
    }
    const discovery = await getCombinedDiscovery();
    if (baselineSourceId && discovery.failures.has(baselineSourceId)) {
      throw discovery.failures.get(baselineSourceId);
    }
    const choices = discovery.choices.filter(
      choice => !baselineSourceId || choice.sourceId === baselineSourceId
    );
    if (choices.length === 0) {
      throw new Error('The selected query sources returned no available queries.');
    }
    const selected = await (signal
      ? selectTree('Select the baseline query', choices, false, signal)
      : selectTree('Select the baseline query', choices, false));
    [baseline] = selectedFromTree(selected, choices);
  } else {
    const source = baselineSourceId
      ? findSource(sources, baselineSourceId)
      : await selectSource(sources, select, 'Select the baseline source', signal);
    const engineId =
      explicitBaselineEngineId ??
      (await selectEngine(source.api, select, 'Select the baseline engine', signal));
    const queryId =
      explicitBaselineQueryId ??
      (await selectQuery(
        source.api,
        select,
        engineId,
        'Select the baseline query',
        new Set(),
        signal
      ));
    baseline = {
      sourceId: source.id,
      sourceLabel: source.label,
      engineId,
      queryId,
    };
  }

  const qualified = ids(values.candidate).map(parseQualifiedCandidate);
  const queryIds = ids(values['candidate-query']);
  const candidateCount = qualified.length + queryIds.length;
  let candidates: SourcedQuerySelection[];
  if (candidateCount > 0) {
    let candidateSources = explicitCandidateSources(
      candidateCount,
      ids(values['candidate-source']),
      sources
    );
    if (!candidateSources) {
      const source = await selectSource(sources, select, 'Select the candidate source', signal);
      candidateSources = Array.from({ length: candidateCount }, () => source);
    }
    candidates = qualified.map((candidate, index) => ({
      ...candidate,
      sourceId: candidateSources[index]!.id,
      sourceLabel: candidateSources[index]!.label,
    }));
    let resolvedEngines = candidateEngineIds(
      queryIds,
      ids(values['candidate-engine']),
      commonEngineId
    );
    if (!resolvedEngines) {
      resolvedEngines = [];
      const selectedEngineBySource = new Map<string, string>();
      for (let index = 0; index < queryIds.length; index += 1) {
        const source = candidateSources[qualified.length + index]!;
        let engineId = selectedEngineBySource.get(source.id);
        if (!engineId) {
          engineId = await selectEngine(
            source.api,
            select,
            `Select the candidate engine from ${source.label}`,
            signal
          );
          selectedEngineBySource.set(source.id, engineId);
        }
        resolvedEngines.push(engineId);
      }
    }
    candidates.push(
      ...queryIds.map((queryId, index) => {
        const source = candidateSources[qualified.length + index]!;
        return {
          sourceId: source.id,
          sourceLabel: source.label,
          engineId: resolvedEngines[index]!,
          queryId,
        };
      })
    );
  } else {
    const requestedSources = ids(values['candidate-source']);
    requestedSources.forEach(id => findSource(sources, id));
    const discovery = await getCombinedDiscovery();
    const failedRequestedSource = requestedSources.find(id => discovery.failures.has(id));
    if (failedRequestedSource) {
      throw discovery.failures.get(failedRequestedSource);
    }
    const choices = discovery.choices.filter(
      choice =>
        (requestedSources.length === 0 || requestedSources.includes(choice.sourceId)) &&
        !(
          choice.sourceId === baseline.sourceId &&
          choice.engineId === baseline.engineId &&
          choice.queryId === baseline.queryId
        )
    );
    if (choices.length === 0) {
      throw new Error('The selected query sources returned no available candidate queries.');
    }
    const selected = await (signal
      ? selectTree('Select candidate queries', choices, true, signal)
      : selectTree('Select candidate queries', choices, true));
    candidates = selectedFromTree(selected, choices);
  }
  return {
    baseline,
    candidates: uniqueSourcedCandidates(baseline, candidates),
  };
}

export function createHttpDiscoveryApi(baseUrl: string, signal?: AbortSignal): DiscoveryApi {
  return createHttpQuerySourceApi(baseUrl, signal);
}

export { resolveDatabaseRunIds };

const defaultQueryDiffRuntime: QueryDiffRuntime = {
  createDiscoveryApi: createHttpDiscoveryApi,
  launchDatabaseRuns: launchQuentOpenDbRuns,
  stopDatabaseRuns: stopQuentOpenDbRuns,
};

export async function runQueryDiffCommand(
  args: string[],
  interaction: QueryDiffInteraction = {
    select: createTerminalSelector(),
    selectTree: selectFromQueryTreeWithInk,
    selectMetrics: selectManyWithInk,
    inputText: inputDatabaseRunIds,
  },
  runtimeOverrides: Partial<QueryDiffRuntime> = {}
): Promise<void> {
  const runtime = { ...defaultQueryDiffRuntime, ...runtimeOverrides };
  const { values } = parseArgs({
    args,
    allowNegative: true,
    allowPositionals: false,
    options: {
      'api-base': { type: 'string' },
      'baseline-engine': { type: 'string' },
      'baseline-query': { type: 'string' },
      'baseline-source': { type: 'string' },
      candidate: { type: 'string', multiple: true },
      'candidate-engine': { type: 'string', multiple: true },
      'candidate-query': { type: 'string', multiple: true },
      'candidate-source': { type: 'string', multiple: true },
      'combined-table': { type: 'boolean', default: true },
      db: { type: 'boolean' },
      'db-api-base-url': { type: 'string' },
      'db-run': { type: 'string', multiple: true },
      'db-token': { type: 'string' },
      'db-trust': { type: 'string', multiple: true },
      'db-trust-all': { type: 'boolean' },
      engine: { type: 'string' },
      json: { type: 'boolean' },
      metric: { type: 'string', multiple: true },
    },
  });

  await withQuerySources(
    values,
    interaction.inputText,
    async ({ sources, databaseMode, signal }) => {
      const { baseline, candidates } = await resolveSourcedQueryDiffSelections(
        values,
        sources,
        interaction.select,
        interaction.selectTree,
        signal
      );
      signal.throwIfAborted();
      const baselineApi = findSource(sources, baseline.sourceId).api;
      const baselineBundle = await baselineApi.fetchQueryBundle(
        baseline.engineId,
        baseline.queryId
      );
      const candidateBundles = await Promise.all(
        candidates.map(candidate =>
          findSource(sources, candidate.sourceId).api.fetchQueryBundle(
            candidate.engineId,
            candidate.queryId
          )
        )
      );
      signal.throwIfAborted();
      const metrics = await resolveQueryDiffMetrics(
        values,
        [baselineBundle, ...candidateBundles],
        interaction.selectMetrics,
        signal
      );
      signal.throwIfAborted();
      const result = diffQueryBundles(
        {
          ...(databaseMode ? { source: baseline.sourceLabel } : {}),
          engineId: baseline.engineId,
          bundle: baselineBundle,
        },
        candidates.map((candidate, index) => ({
          ...(databaseMode ? { source: candidate.sourceLabel } : {}),
          engineId: candidate.engineId,
          bundle: candidateBundles[index]!,
        })),
        { metrics }
      );
      process.stdout.write(
        values.json
          ? serializeMachineOutput(machineResult('query-diff', result))
          : `${formatQueryDiff(result, {
              color: process.stdout.isTTY && !('NO_COLOR' in process.env),
              combinedTable: values['combined-table'],
            })}\n`
      );
    },
    {
      createApi: (baseUrl, signal) => ({
        ...runtime.createDiscoveryApi(baseUrl, signal),
        fetchEntityList: createHttpQuerySourceApi(baseUrl, signal).fetchEntityList,
      }),
      launch: runtime.launchDatabaseRuns,
      stop: runtime.stopDatabaseRuns,
    }
  );
}

export const queryDiffCommand: AskCommand = {
  id: 'query-diff',
  explanation: 'Compares numeric operator statistics and active spans from two query bundles.',
  usage,
  run: runQueryDiffCommand,
};
