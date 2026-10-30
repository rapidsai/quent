// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { parseArgs } from 'node:util';
import {
  createHttpApiClient,
  fetchListEngines,
  fetchListQueries,
  fetchQueryBundle,
  setApiBaseUrl,
} from '@quent/client';
import { commonQueryMetrics, diffQueryBundles } from '@quent/query-analysis';
import type { EntityRef, QueryBundle } from '@quent/utils';
import {
  createNonInteractiveSelector,
  createTerminalSelector,
  discoverQueryTreeChoices,
  selectEngine,
  selectQueriesFromTree,
  selectQuery,
  type DiscoveryApi,
  type QuerySelection,
  type QueryTreeChoice,
  type SelectFromList,
  type SelectFromQueryTree,
} from './askSelection';
import type { AskCommand } from './askCommand.types';
import { inputWithInk, selectFromQueryTreeWithInk, selectManyWithInk } from './inkSelectorRunner';
import { machineResult, serializeMachineOutput } from './machineOutput';
import { launchQuentOpenDbRuns, stopQuentOpenDbRuns, type QuentOpenDbHandle } from './quentOpenDb';
import { formatQueryDiff } from './queryDiffFormat';

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
  --api-base URL          API base (default: QUENT_API_BASE_URL or http://localhost:8080/api)
  --db                    Prompt for comma-separated database run IDs
  --db-run RUN            Database run to add; repeat for multiple runs
  --db-api-base-url URL   Benchmarking API base (default: QUENT_OPEN_API_BASE_URL)
  --db-token TOKEN        Benchmarking API bearer token (default: QUENT_OPEN_TOKEN)
  --db-trust REMOTE       Trust a git remote for quent-open; repeatable
  --db-trust-all          Trust every source opened from the database
  --baseline-source ID    "local" or a registered database run
  --candidate-source ID   Candidate source; one for all or one per candidate
  --json                  Emit versioned JSON; never open interactive prompts`;

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

export interface QueryDiffSelections {
  baseline: QuerySelection;
  candidates: QuerySelection[];
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
  allValue?: string
) => Promise<string[]>;

const ALL_METRICS = '\0all-metrics';

type InputText = (prompt: string, placeholder?: string) => Promise<string>;

export async function resolveQueryDiffMetrics(
  values: QueryDiffCliValues,
  bundles: readonly QueryBundle<EntityRef>[],
  selectMetrics: SelectMetrics = selectManyWithInk
): Promise<string[]> {
  const commonMetrics = commonQueryMetrics(bundles);
  if (commonMetrics.length === 0) {
    throw new Error('The selected queries have no numeric metrics in common.');
  }
  let requested = ids(values.metric);
  if (requested.length === 0) {
    requested = await selectMetrics(
      'Select metrics to compare',
      [
        { value: ALL_METRICS, label: `All metrics (${commonMetrics.length})` },
        ...commonMetrics.map(metric => ({ value: metric, label: metric })),
      ],
      ALL_METRICS
    );
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

function uniqueCandidates(
  baseline: QuerySelection,
  candidates: QuerySelection[]
): QuerySelection[] {
  const baselineKey = `${baseline.engineId}\0${baseline.queryId}`;
  const seen = new Set<string>();
  return candidates.filter(candidate => {
    const key = `${candidate.engineId}\0${candidate.queryId}`;
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
  prompt: string
): Promise<QueryDiffSource> {
  if (sources.length === 1) {
    return sources[0]!;
  }
  const id = await select(
    prompt,
    sources.map(source => ({ value: source.id, label: source.label }))
  );
  return findSource(sources, id);
}

async function discoverSourcedQueryTreeChoices(
  sources: readonly QueryDiffSource[]
): Promise<Array<QueryTreeChoice & { sourceId: string; sourceLabel: string }>> {
  const choices = await Promise.all(
    sources.map(async source =>
      (await discoverQueryTreeChoices(source.api)).map(choice => ({
        ...choice,
        sourceId: source.id,
        sourceLabel: source.label,
        value: `${source.id}\0${choice.value}`,
        engineLabel: `${source.label} · ${choice.engineLabel}`,
      }))
    )
  );
  return choices.flat();
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
  selectTree: SelectFromQueryTree = selectFromQueryTreeWithInk
): Promise<SourcedQueryDiffSelections> {
  if (sources.length === 0) {
    throw new Error('No query sources are available.');
  }
  if (
    sources.length === 1 &&
    sources[0]!.id === 'local' &&
    !values['baseline-source'] &&
    !values['candidate-source']
  ) {
    const selections = await resolveQueryDiffSelections(
      values,
      sources[0]!.api,
      select,
      selectTree
    );
    return {
      baseline: {
        ...selections.baseline,
        sourceId: 'local',
        sourceLabel: 'local',
      },
      candidates: selections.candidates.map(candidate => ({
        ...candidate,
        sourceId: 'local',
        sourceLabel: 'local',
      })),
    };
  }
  const commonEngineId = optionalId(values.engine);
  const baselineSourceId = optionalId(values['baseline-source']);
  const explicitBaselineEngineId = optionalId(values['baseline-engine']) ?? commonEngineId;
  const explicitBaselineQueryId = optionalId(values['baseline-query']);
  let combinedChoices:
    Array<QueryTreeChoice & { sourceId: string; sourceLabel: string }> | undefined;
  const getCombinedChoices = async () => {
    combinedChoices ??= await discoverSourcedQueryTreeChoices(sources);
    return combinedChoices;
  };

  let baseline: SourcedQuerySelection;
  if (!explicitBaselineEngineId && !explicitBaselineQueryId) {
    if (baselineSourceId) {
      findSource(sources, baselineSourceId);
    }
    const choices = (await getCombinedChoices()).filter(
      choice => !baselineSourceId || choice.sourceId === baselineSourceId
    );
    if (choices.length === 0) {
      throw new Error('The selected query sources returned no available queries.');
    }
    const selected = await selectTree('Select the baseline query', choices, false);
    [baseline] = selectedFromTree(selected, choices);
  } else {
    const source = baselineSourceId
      ? findSource(sources, baselineSourceId)
      : await selectSource(sources, select, 'Select the baseline source');
    const engineId =
      explicitBaselineEngineId ??
      (await selectEngine(source.api, select, 'Select the baseline engine'));
    const queryId =
      explicitBaselineQueryId ??
      (await selectQuery(source.api, select, engineId, 'Select the baseline query'));
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
      const source = await selectSource(sources, select, 'Select the candidate source');
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
      for (let index = 0; index < queryIds.length; index += 1) {
        const source = candidateSources[qualified.length + index]!;
        resolvedEngines.push(
          await selectEngine(source.api, select, `Select the candidate engine from ${source.label}`)
        );
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
    const choices = (await getCombinedChoices()).filter(
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
    const selected = await selectTree('Select candidate queries', choices, true);
    candidates = selectedFromTree(selected, choices);
  }
  return {
    baseline,
    candidates: uniqueSourcedCandidates(baseline, candidates),
  };
}

export async function resolveQueryDiffSelections(
  values: QueryDiffCliValues,
  api: DiscoveryApi,
  select: SelectFromList,
  selectTree: SelectFromQueryTree = selectFromQueryTreeWithInk
): Promise<QueryDiffSelections> {
  const commonEngineId = optionalId(values.engine);
  let treeChoices: Awaited<ReturnType<typeof discoverQueryTreeChoices>> | undefined;
  const getTreeChoices = async () => {
    treeChoices ??= await discoverQueryTreeChoices(api);
    return treeChoices;
  };
  const explicitBaselineEngineId = optionalId(values['baseline-engine']) ?? commonEngineId;
  const explicitBaselineQueryId = optionalId(values['baseline-query']);
  let baseline: QuerySelection;
  if (!explicitBaselineEngineId && !explicitBaselineQueryId) {
    [baseline] = await selectQueriesFromTree(
      api,
      selectTree,
      'Select the baseline query',
      false,
      new Set(),
      await getTreeChoices()
    );
  } else {
    const engineId =
      explicitBaselineEngineId ?? (await selectEngine(api, select, 'Select the baseline engine'));
    const queryId =
      explicitBaselineQueryId ??
      (await selectQuery(api, select, engineId, 'Select the baseline query'));
    baseline = { engineId, queryId };
  }

  const qualified = ids(values.candidate).map(parseQualifiedCandidate);
  const queryIds = ids(values['candidate-query']);
  const engines = ids(values['candidate-engine']);
  let candidates = qualified;
  if (queryIds.length > 0) {
    let resolvedEngines = candidateEngineIds(queryIds, engines, commonEngineId);
    if (!resolvedEngines) {
      const engineId = await selectEngine(api, select, 'Select the candidate engine');
      resolvedEngines = queryIds.map(() => engineId);
    }
    candidates = [
      ...candidates,
      ...queryIds.map((queryId, index) => ({
        engineId: resolvedEngines[index]!,
        queryId,
      })),
    ];
  }
  if (candidates.length === 0) {
    candidates = await selectQueriesFromTree(
      api,
      selectTree,
      'Select candidate queries',
      true,
      new Set([`${baseline.engineId}\0${baseline.queryId}`]),
      await getTreeChoices()
    );
  }
  return { baseline, candidates: uniqueCandidates(baseline, candidates) };
}

async function inputDatabaseRunIds(prompt: string, placeholder?: string): Promise<string> {
  if (!process.stdin.isTTY || !process.stderr.isTTY) {
    throw new Error('Database run selection requires an interactive terminal; pass --db-run.');
  }
  return inputWithInk(prompt, placeholder);
}

export async function resolveDatabaseRunIds(
  values: QueryDiffCliValues,
  inputText: InputText = inputDatabaseRunIds
): Promise<string[]> {
  const explicit = ids(values['db-run']);
  if (explicit.length > 0) {
    return [...new Set(explicit)];
  }
  if (!values.db) {
    return [];
  }
  if (values.json) {
    throw new Error('JSON mode requires explicit --db-run when --db is used.');
  }
  const entered = await inputText('Enter database run IDs', 'comma-separated, e.g. 6647, 6650');
  const runs = entered
    .split(',')
    .map(run => run.trim())
    .filter(Boolean);
  if (runs.length === 0) {
    throw new Error('Enter at least one database run ID.');
  }
  return [...new Set(runs)];
}

function globalDiscoveryApi(): DiscoveryApi {
  return {
    fetchListEngines,
    fetchListQueries,
    fetchQueryBundle,
  };
}

function clientDiscoveryApi(baseUrl: string): DiscoveryApi {
  const client = createHttpApiClient(baseUrl);
  return {
    fetchListEngines: client.fetchListEngines,
    fetchListQueries: client.fetchListQueries,
    fetchQueryBundle: client.fetchQueryBundle,
  };
}

export const queryDiffCommand: AskCommand = {
  id: 'query-diff',
  explanation: 'Compares numeric operator statistics and active spans from two query bundles.',
  usage,
  async run(args) {
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

    const configuredLocalApiBase = values['api-base'] ?? process.env.QUENT_API_BASE_URL;
    const apiBase = configuredLocalApiBase ?? 'http://localhost:8080/api';
    setApiBaseUrl(apiBase.replace(/\/+$/u, ''));
    const databaseRuns = await resolveDatabaseRunIds(values);
    const databaseMode = Boolean(values.db) || databaseRuns.length > 0;
    const includeLocal = !databaseMode || configuredLocalApiBase !== undefined;

    if (values.json) {
      const qualifiedCount = ids(values.candidate).length;
      const candidateQueryCount = ids(values['candidate-query']).length;
      const hasCandidates = qualifiedCount > 0 || candidateQueryCount > 0;
      const candidateQueriesHaveEngine =
        candidateQueryCount === 0 ||
        ids(values['candidate-engine']).length > 0 ||
        Boolean(values.engine);
      const sourceCount = databaseRuns.length + (includeLocal ? 1 : 0);
      const missing = [
        !values.engine && !values['baseline-engine'] && '--baseline-engine or --engine',
        !values['baseline-query'] && '--baseline-query',
        !hasCandidates && '--candidate or --candidate-query',
        hasCandidates && !candidateQueriesHaveEngine && '--candidate-engine or --engine',
        sourceCount > 1 && !values['baseline-source'] && '--baseline-source',
        sourceCount > 1 && ids(values['candidate-source']).length === 0 && '--candidate-source',
        ids(values.metric).length === 0 && '--metric',
      ].filter((option): option is string => Boolean(option));
      if (missing.length > 0) {
        throw new Error(
          `JSON mode requires explicit ${missing.join(', ')}. Use the discovery commands to find IDs.`
        );
      }
    }

    const select = values.json ? createNonInteractiveSelector() : createTerminalSelector();
    const selectTree: SelectFromQueryTree = values.json
      ? async () => {
          throw new Error('Query tree selection is unavailable in JSON mode.');
        }
      : selectFromQueryTreeWithInk;
    let handles: QuentOpenDbHandle[] = [];
    let interrupted = false;
    const stopOnSignal = (exitCode: number) => {
      interrupted = true;
      process.exitCode = exitCode;
      void stopQuentOpenDbRuns(handles);
    };
    const onInterrupt = () => stopOnSignal(130);
    const onTerminate = () => stopOnSignal(143);
    if (databaseMode) {
      process.once('SIGINT', onInterrupt);
      process.once('SIGTERM', onTerminate);
    }
    try {
      if (databaseMode) {
        handles = await launchQuentOpenDbRuns(databaseRuns, {
          apiBaseUrl: optionalId(values['db-api-base-url']),
          interactive: !values.json,
          token: optionalId(values['db-token']),
          trust: ids(values['db-trust']),
          trustAll: Boolean(values['db-trust-all']),
        });
      }
      if (interrupted) {
        return;
      }
      const sources: QueryDiffSource[] = [
        ...(includeLocal
          ? [
              {
                id: 'local',
                label: 'local',
                api: databaseMode
                  ? clientDiscoveryApi(apiBase.replace(/\/+$/u, ''))
                  : globalDiscoveryApi(),
              },
            ]
          : []),
        ...handles.map(handle => ({
          id: handle.run,
          label: `db ${handle.run}`,
          api: clientDiscoveryApi(handle.apiBaseUrl),
        })),
      ];
      const { baseline, candidates } = await resolveSourcedQueryDiffSelections(
        values,
        sources,
        select,
        selectTree
      );
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
      const metrics = await resolveQueryDiffMetrics(values, [baselineBundle, ...candidateBundles]);
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
    } finally {
      if (databaseMode) {
        process.removeListener('SIGINT', onInterrupt);
        process.removeListener('SIGTERM', onTerminate);
        await stopQuentOpenDbRuns(handles);
      }
    }
  },
};
