<!-- SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script lang="ts">
  import { onMount } from 'svelte';
  import BenchmarkPlot from './BenchmarkPlot.svelte';
  import type { BenchCase, BenchLanguage, BenchReport } from './report';
  import { availableLanguages, caseLanguage, frameworkColors, hasDiscardedCalls, isNoopCase, languageLabel, payloadGroups, sortCasesByAverage } from './report';
  import { loadPlotPreference, plotPreference, setIncludeDiscarded, setIncludeNoop, setPlotMode } from './plotPreference.svelte';
  import badgerUrl from '../../../../ui/public/logo.svg';

  interface Selection { current: { url: string; version: string } | null }
  let current = $state<BenchReport | null>(null);
  let currentUrl = $state<string | null>(null);
  let error = $state('');
  let loading = $state(true);
  let selectedLanguage = $state<BenchLanguage>('rust');
  let excludedThreads = $state<number[]>([]);
  let busy = false;
  let selectionVersion = '';

  let languages = $derived(current ? availableLanguages(current) : []);
  let discardedCases = $derived(current?.cases.filter((item) =>
    caseLanguage(item) === selectedLanguage && hasDiscardedCalls(item)) ?? []);
  let visibleDiscarded = $derived(discardedCases.some((item) =>
    (plotPreference.includeNoop || !isNoopCase(item)) && !excludedThreads.includes(item.threads)));
  let colors = $derived(frameworkColors(current?.cases.map((item) => item.implementation) ?? []));
  let groups = $derived(current ? payloadGroups(current, selectedLanguage, plotPreference.includeNoop,
    plotPreference.includeDiscarded) : []);
  let emptyLoops = $derived(current?.cases.filter((item) => item.event_shape === null &&
    caseVisible(item)) ?? []);
  let threadCounts = $derived([...new Set(current?.cases.filter((item) =>
    caseVisible(item)).map((item) => item.threads) ?? [])].sort((a, b) => a - b));
  let visibleGroups = $derived(groups.map((group) => ({ ...group,
    threads: group.threads.filter((threads) => !excludedThreads.includes(threads)),
  })).filter((group) => group.threads.length > 0));
  let visibleEmptyLoops = $derived(emptyLoops.filter((item) => !excludedThreads.includes(item.threads)));

  function setThreadVisible(threads: number, visible: boolean): void {
    excludedThreads = visible ? excludedThreads.filter((value) => value !== threads) : [...excludedThreads, threads];
  }

  function caseVisible(item: BenchCase): boolean {
    return caseLanguage(item) === selectedLanguage &&
      (plotPreference.includeNoop || !isNoopCase(item)) &&
      (plotPreference.includeDiscarded || !hasDiscardedCalls(item));
  }

  function currentCases(payload: string, threads: number): BenchCase[] {
    return sortCasesByAverage(current?.cases.filter((item) => caseVisible(item) &&
      item.event_shape === payload && item.threads === threads) ?? []);
  }

  async function refresh() {
    if (busy) return;
    busy = true;
    try {
      const embeddedReport = document.getElementById('quent-bench-report')?.textContent;
      if (embeddedReport) {
        const report = JSON.parse(embeddedReport) as BenchReport;
        const reportLanguages = availableLanguages(report);
        if (!reportLanguages.includes(selectedLanguage)) selectedLanguage = reportLanguages[0] ?? 'rust';
        current = report;
        currentUrl = URL.createObjectURL(new Blob([embeddedReport], { type: 'application/json' }));
        error = '';
        return;
      }
      const selectionUrl = import.meta.env.DEV ? '/api/selection' : './selection.json';
      const response = await fetch(selectionUrl, { cache: 'no-store' });
      if (!response.ok) throw new Error(`Report list: HTTP ${response.status}`);
      const selection: Selection = await response.json();
      const version = JSON.stringify(selection);
      if (version === selectionVersion) return;
      if (!selection.current) {
        current = null; currentUrl = null;
      } else {
        const reportResponse = await fetch(selection.current.url, { cache: 'no-store' });
        if (!reportResponse.ok) throw new Error(`Report: HTTP ${reportResponse.status}`);
        const report = await reportResponse.json() as BenchReport;
        const reportLanguages = availableLanguages(report);
        if (!reportLanguages.includes(selectedLanguage)) selectedLanguage = reportLanguages[0] ?? 'rust';
        current = report;
        currentUrl = selection.current.url;
      }
      selectionVersion = version;
      error = '';
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      loading = false;
      busy = false;
    }
  }

  onMount(() => {
    loadPlotPreference();
    void refresh();
    if (import.meta.env.DEV) {
      const timer = setInterval(() => void refresh(), 2500);
      return () => { clearInterval(timer); if (currentUrl?.startsWith('blob:')) URL.revokeObjectURL(currentUrl); };
    }
    return () => { if (currentUrl?.startsWith('blob:')) URL.revokeObjectURL(currentUrl); };
  });

  function date(seconds: number): string { return new Date(seconds * 1000).toLocaleString(); }
  function shortCommit(commit: string | null): string { return commit ? commit.slice(0, 12) : 'Unavailable'; }
</script>

<svelte:head>
  <title>Quent Benchmark Plot</title>
  <meta name="description" content="Quent instrumentation latency benchmark results" />
</svelte:head>

<div class="min-h-screen bg-base-200 text-base-content">
  <header class="flex min-w-0 flex-wrap items-center gap-3 p-4 sm:px-6">
    <img class="h-8 w-8 shrink-0 opacity-70" src={badgerUrl} alt="Quent badger" />
    <h1 class="truncate text-xl font-semibold">Quent Benchmarks</h1>
    <div class="ml-auto flex min-w-0 max-w-full flex-wrap items-center gap-3">
      {#if current}
        <div class="badge h-auto max-w-full flex-col items-start border-transparent bg-base-content/5 px-2 py-1 text-base-content/40">
          <span class="max-w-full break-all font-mono text-xs leading-none whitespace-normal">Quent {shortCommit(current.system.git_commit)}{current.system.git_dirty ? ' (dirty)' : ''}</span>
          <time class="text-xs leading-none" datetime={new Date(current.system.captured_at_unix_seconds * 1000).toISOString()} title="Report time in your local timezone">report: {date(current.system.captured_at_unix_seconds)}</time>
        </div>
        {#if currentUrl}<a class="btn btn-sm" href={currentUrl} download>Download JSON</a>{/if}
      {/if}
    </div>
  </header>
  <div class="tabs tabs-border px-4 sm:px-6" role="tablist" aria-label="Measurement type">
    <button id="latency-tab" class="tab tab-active" type="button" role="tab" aria-selected="true" aria-controls="latency-panel">Instrumentation latency</button>
  </div>
  <main class="w-full px-4 py-6 sm:px-6">
    <div id="latency-panel" role="tabpanel" aria-labelledby="latency-tab">
    <p class="leading-relaxed text-base-content/70">Batch-average time per instrumentation call. Lower latency is better.</p>
    <div class="mt-5 flex flex-wrap items-center gap-x-8 gap-y-4">
      <label class="flex cursor-pointer items-center gap-3">
        <span>Simple plot</span>
        <input type="checkbox" class="toggle toggle-primary" role="switch" aria-label="Advanced plot" checked={plotPreference.mode === 'advanced'} onchange={(event) => setPlotMode(event.currentTarget.checked ? 'advanced' : 'simple')} />
        <span>Advanced plot</span>
      </label>
      {#if current?.cases.some(hasDiscardedCalls)}
        <label class="flex cursor-pointer items-center gap-3">
          <input type="checkbox" class="toggle toggle-warning" role="switch" checked={plotPreference.includeDiscarded} onchange={(event) => setIncludeDiscarded(event.currentTarget.checked)} />
          <span>Include discarded</span>
        </label>
      {/if}
      {#if plotPreference.includeDiscarded && current?.cases.some(isNoopCase)}
        <label class="flex cursor-pointer items-center gap-3">
          <input type="checkbox" class="toggle toggle-secondary" role="switch" checked={plotPreference.includeNoop} onchange={(event) => setIncludeNoop(event.currentTarget.checked)} />
          <span>Include noop</span>
        </label>
      {/if}
      {#if current}
        <fieldset class="flex flex-wrap items-center gap-4">
          <legend class="sr-only">Language</legend>
          {#each languages as language}
            <label class="flex cursor-pointer items-center gap-2">
              <input type="radio" class="radio radio-secondary radio-sm" name="language" value={language} bind:group={selectedLanguage} />
              <span>{languageLabel(language)}</span>
            </label>
          {/each}
        </fieldset>
      {/if}
    </div>
    {#if discardedCases.length && plotPreference.includeDiscarded}
      <p class="mt-3 text-sm text-base-content/70">⚠ and muted colors mark cases with discarded calls.{#if !visibleDiscarded} The current filters hide all such cases.{/if}</p>
    {/if}
    {#if threadCounts.length > 1}
      <fieldset class="mt-5">
        <legend class="mb-2 text-sm font-medium">Threads</legend>
        <div class="filter" aria-label="Thread count filter">
          {#each threadCounts as threads}
            <input type="checkbox" class="btn btn-sm" aria-label={`${threads} ${threads === 1 ? 'thread' : 'threads'}`}
              checked={!excludedThreads.includes(threads)}
              onchange={(event) => setThreadVisible(threads, event.currentTarget.checked)} />
          {/each}
        </div>
      </fieldset>
    {/if}
  {#if loading}
    <p class="flex items-center gap-3" role="status"><span class="loading loading-spinner loading-sm"></span>Loading reports…</p>
  {:else if error}
    <p class="alert alert-error" role="alert">Could not load reports: {error}</p>
  {:else if !current}
    <p class="alert alert-info" role="status">No benchmark reports found. Run <code>quent-bench</code> to create one.</p>
  {:else}
    <section class="mb-10 grid gap-3 md:grid-cols-2" aria-label="Run details">
      <div class="card card-border bg-base-100 shadow-sm"><div class="card-body gap-2 p-5"><span class="badge badge-outline">Environment</span><strong>{current.system.cpu_model ?? 'Unknown CPU'}</strong><small class="text-base-content/60">{current.system.os} · {current.system.architecture} · {current.system.available_cpu_count ?? '?'} available CPUs</small>{#if selectedLanguage === 'rust'}<small class="text-base-content/60">{current.system.rustc_version ?? 'Rust version unavailable'}</small>{/if}</div></div>
      <div class="card card-border bg-base-100 shadow-sm"><div class="card-body gap-2 p-5"><span class="badge badge-outline">Measurement</span><strong>{current.cases[0]?.num_batches} batches × {current.cases[0]?.batch_size} calls</strong><small class="text-base-content/60">Warmup: {current.cases[0]?.num_warmup_batches} batches · pause: {current.cases[0]?.batch_pause_interval_us} µs</small></div></div>
    </section>
    {#if !visibleGroups.length && !visibleEmptyLoops.length}
      <p class="alert alert-info" role="status">No benchmark cases match the selected filters.</p>
    {/if}
    {#each visibleGroups as group}
      <section class="mt-11" aria-label={`${group.name} payload`}>
        <h2 class="mb-5 text-2xl font-semibold">{group.name} payload</h2>
        <div class="flex gap-4 overflow-x-auto pb-3" role="region" aria-label={`${group.name} payload thread plots`}>
          {#each group.threads as threads}
            {@const plotCases = currentCases(group.name, threads)}
            <div class="shrink-0" style:width={`${Math.max(360, plotCases.length * 48 + 140)}px`}>
              <BenchmarkPlot cases={plotCases} title={`${threads} ${threads === 1 ? 'thread' : 'threads'}`} mode={plotPreference.mode} {colors} />
            </div>
          {/each}
        </div>
      </section>
    {/each}
    {#if visibleEmptyLoops.length}
      <section class="mt-11" aria-label="Empty loop reference">
        <h2 class="mb-2 text-2xl font-semibold">Empty loop reference</h2>
        <p class="mb-5 text-base-content/70">Loop and timing overhead is shown separately and is not subtracted from instrumentation results.</p>
        <div class="flex gap-4 overflow-x-auto pb-3" role="region" aria-label="Empty loop thread plots">
          {#each visibleEmptyLoops as item}
            <div class="w-[360px] shrink-0">
              <BenchmarkPlot cases={[item]} title={`${item.threads} ${item.threads === 1 ? 'thread' : 'threads'}`} mode={plotPreference.mode} {colors} />
            </div>
          {/each}
        </div>
      </section>
    {/if}
  {/if}
    </div>
  </main>
</div>
