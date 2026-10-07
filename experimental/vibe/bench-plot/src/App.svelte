<!-- SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script lang="ts">
  import { onMount } from 'svelte';
  import BenchmarkPlot from './BenchmarkPlot.svelte';
  import type { BenchCase, BenchLanguage, BenchReport } from './report';
  import { availableLanguages, caseLanguage, frameworkColors, hasDiscardedCalls, isNoopCase, languageLabel, payloadGroups, sortCasesByAverage } from './report';
  import { loadPlotPreference, plotPreference, setIncludeDiscarded, setIncludeNoop, setPlotMode } from './plotPreference.svelte';
  import badgerUrl from '../../../../ui/public/logo.svg';

  const payloadColumnWidth = 72;
  interface Selection { current: { url: string; version: string } | null }
  let current = $state<BenchReport | null>(null);
  let currentUrl = $state<string | null>(null);
  let error = $state('');
  let loading = $state(true);
  let excludedLanguages = $state<BenchLanguage[]>([]);
  let excludedThreads = $state<number[]>([]);
  let plotGridElement = $state<HTMLDivElement | undefined>(undefined);
  let plotScrollbarElement = $state<HTMLDivElement | undefined>(undefined);
  let plotScrollLeft = $state(0);
  let canScrollLeft = $state(false);
  let canScrollRight = $state(false);
  let busy = false;
  let selectionVersion = '';

  let languages = $derived(current ? availableLanguages(current) : []);
  let selectedLanguages = $derived(languages.filter((language) => !excludedLanguages.includes(language)));
  let discardedCases = $derived(current?.cases.filter((item) =>
    selectedLanguages.includes(caseLanguage(item)) && hasDiscardedCalls(item)) ?? []);
  let visibleDiscarded = $derived(discardedCases.some((item) =>
    (plotPreference.includeNoop || !isNoopCase(item)) && !excludedThreads.includes(item.threads)));
  let colors = $derived(frameworkColors(current?.cases.map((item) => item.implementation) ?? []));
  let groups = $derived(current ? payloadGroups(current, selectedLanguages, plotPreference.includeNoop,
    plotPreference.includeDiscarded) : []);
  let emptyLoops = $derived(current?.cases.filter((item) => item.event_shape === null &&
    caseVisible(item)) ?? []);
  let threadCounts = $derived([...new Set(current?.cases.filter((item) =>
    caseVisible(item)).map((item) => item.threads) ?? [])].sort((a, b) => a - b));
  let visibleGroups = $derived(groups.map((group) => ({ ...group,
    threads: group.threads.filter((threads) => !excludedThreads.includes(threads)),
  })).filter((group) => group.threads.length > 0));
  let visibleEmptyLoops = $derived(emptyLoops.filter((item) => !excludedThreads.includes(item.threads)));
  let visibleThreadCounts = $derived(threadCounts.filter((threads) => !excludedThreads.includes(threads)));
  let threadWidths = $derived(new Map(visibleThreadCounts.map((threads) => [threads,
    Math.max(320, ...visibleGroups.filter((group) => group.threads.includes(threads))
      .map((group) => currentCases(group.name, threads).length * 44 + 110))])));
  let plotWidth = $derived(payloadColumnWidth + [...threadWidths.values()].reduce((sum, width) => sum + width, 0));

  function threadStops(): number[] {
    let offset = 0;
    return visibleThreadCounts.map((threads) => {
      const stop = offset;
      offset += threadWidths.get(threads) ?? 320;
      return stop;
    });
  }

  function updateScrollButtons(): void {
    if (!plotGridElement) return;
    canScrollLeft = plotGridElement.scrollLeft > 1;
    canScrollRight = plotGridElement.scrollLeft < plotGridElement.scrollWidth - plotGridElement.clientWidth - 1;
  }

  function syncGridScroll(): void {
    if (!plotGridElement) return;
    plotScrollLeft = plotGridElement.scrollLeft;
    if (plotScrollbarElement && Math.abs(plotScrollbarElement.scrollLeft - plotScrollLeft) > 1) {
      plotScrollbarElement.scrollLeft = plotScrollLeft;
    }
    updateScrollButtons();
  }

  function syncScrollbarScroll(): void {
    if (!plotGridElement || !plotScrollbarElement) return;
    const position = plotScrollbarElement.scrollLeft;
    if (Math.abs(plotGridElement.scrollLeft - position) > 1) plotGridElement.scrollLeft = position;
    plotScrollLeft = position;
    updateScrollButtons();
  }

  function setPlotScroll(position: number): void {
    if (!plotGridElement) return;
    const max = plotGridElement.scrollWidth - plotGridElement.clientWidth;
    const target = Math.max(0, Math.min(position, max));
    plotGridElement.scrollLeft = target;
    if (plotScrollbarElement) plotScrollbarElement.scrollLeft = target;
    plotScrollLeft = target;
    updateScrollButtons();
  }

  function scrollThread(direction: -1 | 1): void {
    if (!plotGridElement) return;
    const position = plotGridElement.scrollLeft;
    const stops = threadStops();
    const target = direction === 1
      ? stops.find((stop) => stop > position + 2)
      : stops.findLast((stop) => stop < position - 2);
    setPlotScroll(target ?? (direction === 1 ? plotGridElement.scrollWidth : 0));
  }

  $effect(() => {
    threadStops();
    if (!plotGridElement) return;
    const frame = requestAnimationFrame(syncGridScroll);
    const observer = new ResizeObserver(syncGridScroll);
    observer.observe(plotGridElement);
    return () => { cancelAnimationFrame(frame); observer.disconnect(); };
  });

  function setThreadVisible(threads: number, visible: boolean): void {
    excludedThreads = visible ? excludedThreads.filter((value) => value !== threads) : [...excludedThreads, threads];
  }

  function setLanguageVisible(language: BenchLanguage, visible: boolean): void {
    excludedLanguages = visible ? excludedLanguages.filter((value) => value !== language) : [...excludedLanguages, language];
  }

  function caseVisible(item: BenchCase): boolean {
    return selectedLanguages.includes(caseLanguage(item)) &&
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
  <main class="w-full px-4 py-4 sm:px-6">
    <div id="latency-panel" role="tabpanel" aria-labelledby="latency-tab">
    <p class="leading-relaxed text-base-content/70">Batch-average time per instrumentation call. Lower latency is better. Plots are ordered by average latency; values above boxes or bars show average ns/call.</p>
    <div class="mt-3 flex flex-wrap items-center gap-x-6 gap-y-2">
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
          <legend class="sr-only">Languages</legend>
          {#each languages as language}
            <label class="flex cursor-pointer items-center gap-2">
              <input type="checkbox" class="toggle toggle-secondary toggle-sm" role="switch"
                checked={selectedLanguages.includes(language)}
                onchange={(event) => setLanguageVisible(language, event.currentTarget.checked)} />
              <span>{languageLabel(language)}</span>
            </label>
          {/each}
        </fieldset>
      {/if}
    </div>
    {#if discardedCases.length && plotPreference.includeDiscarded}
      <p class="mt-3 text-sm text-base-content/70">⚠ and muted colors mark cases with discarded calls.{#if plotPreference.includeNoop && discardedCases.some(isNoopCase)} Gray bands mark fully discarded cases.{/if}{#if !visibleDiscarded} The current filters hide all such cases.{/if}</p>
    {/if}
    {#if threadCounts.length > 1}
      <fieldset class="mt-3">
        <legend class="mb-1 text-sm font-medium">Threads</legend>
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
    <section class="mb-4 grid gap-2 md:grid-cols-2" aria-label="Run details">
      <div class="card card-border bg-base-100 shadow-sm"><div class="card-body gap-1 p-3"><span class="badge badge-outline">Environment</span><strong>{current.system.cpu_model ?? 'Unknown CPU'}</strong><small class="text-base-content/60">{current.system.os} · {current.system.architecture} · {current.system.available_cpu_count ?? '?'} available CPUs</small>{#if selectedLanguages.includes('rust')}<small class="text-base-content/60">{current.system.rustc_version ?? 'Rust version unavailable'}</small>{/if}</div></div>
      <div class="card card-border bg-base-100 shadow-sm"><div class="card-body gap-1 p-3"><span class="badge badge-outline">Measurement</span><strong>{current.cases[0]?.num_batches} batches × {current.cases[0]?.batch_size} calls</strong><small class="text-base-content/60">Warmup: {current.cases[0]?.num_warmup_batches} batches · pause: {current.cases[0]?.batch_pause_interval_us} µs</small></div></div>
    </section>
    {#if !visibleGroups.length && !visibleEmptyLoops.length}
      <p class="alert alert-info" role="status">No benchmark cases match the selected filters.</p>
    {/if}
    {#if visibleGroups.length || visibleEmptyLoops.length}
      <div class="mt-4 flex flex-wrap items-center justify-between gap-2">
        <div>
          <h2 class="text-base font-semibold">Results by payload</h2>
          <p class="text-xs text-base-content/60">Producer threads across columns</p>
        </div>
        <div class="flex items-center gap-2">
          <span class="text-sm font-medium text-base-content/70">Threads</span>
          <div class="join" role="group" aria-label="Thread column navigation">
            <button type="button" class="btn btn-sm join-item" aria-label="Previous thread column" title="Previous thread column"
              disabled={!canScrollLeft} onclick={() => scrollThread(-1)}>
              <svg aria-hidden="true" class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m15 18-6-6 6-6" /></svg>
            </button>
            <button type="button" class="btn btn-sm join-item" aria-label="Next thread column" title="Next thread column"
              disabled={!canScrollRight} onclick={() => scrollThread(1)}>
              <svg aria-hidden="true" class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m9 18 6-6-6-6" /></svg>
            </button>
          </div>
        </div>
      </div>
      <div class="relative mt-2">
        <div class="sticky -top-px z-30 h-11 overflow-hidden rounded-t-box border border-base-300 bg-base-200 shadow-sm" aria-hidden="true">
          <div class="absolute inset-y-0 left-0 z-10 flex items-center border-r border-base-300 bg-base-200 px-2 text-xs font-semibold uppercase tracking-wide text-base-content/60" style:width={`${payloadColumnWidth}px`}>Payload</div>
          <div class="flex h-full" style:transform={`translateX(${payloadColumnWidth - plotScrollLeft}px)`}>
            {#each visibleThreadCounts as threads}
              <div class="flex h-full shrink-0 items-center gap-2 border-r border-base-300 px-3" style:width={`${threadWidths.get(threads) ?? 320}px`}>
                <span class="text-base font-semibold">{threads}</span><span class="text-sm font-medium text-base-content/70">{threads === 1 ? 'thread' : 'threads'}</span>
              </div>
            {/each}
          </div>
        </div>
        <!-- svelte-ignore a11y_no_noninteractive_tabindex: the plot grid must be keyboard-scrollable -->
        <div class="plot-grid-body overflow-x-auto overflow-y-hidden border-x border-base-300 bg-base-100" role="region" aria-label="Instrumentation latency plots" tabindex="0" bind:this={plotGridElement} onscroll={syncGridScroll}>
        <table class="table-fixed border-collapse" style:width={`${plotWidth}px`}>
          <colgroup>
            <col style:width={`${payloadColumnWidth}px`} />
            {#each visibleThreadCounts as threads}
              <col style:width={`${threadWidths.get(threads) ?? 320}px`} />
            {/each}
          </colgroup>
          <thead class="sr-only">
            <tr>
              <th scope="col">Payload</th>
              {#each visibleThreadCounts as threads}
                <th scope="col">{threads} {threads === 1 ? 'thread' : 'threads'}</th>
              {/each}
            </tr>
          </thead>
          <tbody>
            {#each visibleGroups as group}
              <tr>
                <th scope="row" class="sticky left-0 z-10 break-words border border-base-300 bg-base-200 p-2 text-left align-middle text-sm font-semibold [overflow-wrap:anywhere]">{group.name}</th>
                {#each visibleThreadCounts as threads}
                  <td class="border border-base-300 align-top">
                    {#if group.threads.includes(threads)}
                      <BenchmarkPlot cases={currentCases(group.name, threads)} title={`${group.name} payload, ${threads} ${threads === 1 ? 'thread' : 'threads'}`} mode={plotPreference.mode} {colors} />
                    {/if}
                  </td>
                {/each}
              </tr>
            {/each}
            {#if visibleEmptyLoops.length}
              <tr>
                <th scope="row" class="sticky left-0 z-10 break-words border border-base-300 bg-base-200 p-2 text-left align-middle [overflow-wrap:anywhere]">
                  <span class="text-sm font-semibold">Empty loop</span>
                  <span class="mt-1 block text-xs font-normal text-base-content/60">Timing overhead, not subtracted</span>
                </th>
                {#each visibleThreadCounts as threads}
                  {@const items = sortCasesByAverage(visibleEmptyLoops.filter((loop) => loop.threads === threads))}
                  <td class="border border-base-300 align-top">
                    {#if items.length}
                      <BenchmarkPlot cases={items} title={`Empty loop, ${threads} ${threads === 1 ? 'thread' : 'threads'}`} mode={plotPreference.mode} {colors} />
                    {/if}
                  </td>
                {/each}
              </tr>
            {/if}
          </tbody>
        </table>
        </div>
        <div class="plot-grid-scroll sticky bottom-0 z-30 h-5 overflow-x-scroll overflow-y-hidden rounded-b-box border border-base-300 bg-base-200" bind:this={plotScrollbarElement} onscroll={syncScrollbarScroll}>
          <div class="h-px" style:width={`${plotWidth}px`}></div>
        </div>
      </div>
    {/if}
  {/if}
    </div>
  </main>
</div>
