<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script lang="ts">
  import '@quent/schema-viewer';
  import '@quent/schema-viewer/styles.css';
  import {
    DEFAULT_ENTITY_GRAPH_CONFIG,
    parseFsm,
    parseResource,
    samePath,
    type EntityGraphView,
    type EntityGraphViewChange,
    type ResolvedEntityGraphConfig,
    type Schema,
    type SchemaPath,
    type SchemaSelection,
  } from '@quent/schema-viewer';
  import {
    Pane,
    PaneGroup,
    PaneResizer,
    type PaneAPI,
  } from 'paneforge';

  import EntityNode from './EntityNode.svelte';
  import GraphConfigBar from './GraphConfigBar.svelte';
  import SelectionBreadcrumbs from './SelectionBreadcrumbs.svelte';
  import YamlEditor from './YamlEditor.svelte';
  import {
    yamlExampleModels,
    type ExampleModelId,
  } from './yaml-models';
  import {
    parseYamlSchema,
    parserErrorMessage,
  } from './yaml-schema';
  import {
    observeTheme,
    readThemePreference,
    saveThemePreference,
    type ThemePreference,
  } from './theme';

  type ModelSelectionId = ExampleModelId | 'loaded';

  const initialModel =
    yamlExampleModels.find(
      ({ id }) =>
        id === new URLSearchParams(window.location.search).get('example'),
    ) ?? yamlExampleModels[0]!;

  let modelId = $state<ModelSelectionId>(initialModel.id);
  let loadedFileName = $state<string | null>(null);
  let yamlFileInput = $state<HTMLInputElement | null>(null);
  let yamlSource = $state(initialModel.source);
  let schema = $state<Schema | null>(null);
  let parserStatus = $state<'parsing' | 'valid' | 'invalid'>('parsing');
  let parserError = $state<string | null>(null);
  let selection = $state<SchemaSelection | null>(null);
  let hoverSelection = $state<SchemaSelection | null>(null);
  let breadcrumbPreview = $state<SchemaSelection | null>(null);
  let activeView = $state<EntityGraphView>('graph');
  let graphExpanded = $state(false);
  let editorCollapsed = $state(false);
  let graphCollapsed = $state(false);
  let selectionsCollapsed = $state(false);
  let editorPane = $state<PaneAPI | null>(null);
  let graphPane = $state<PaneAPI | null>(null);
  let selectionsPane = $state<PaneAPI | null>(null);
  let themePreference = $state<ThemePreference>(readThemePreference());
  let config = $state<ResolvedEntityGraphConfig>({
    ...DEFAULT_ENTITY_GRAPH_CONFIG,
  });
  let tooltipSelection = $derived(hoverSelection ?? selection);
  let paneSelection = $derived(breadcrumbPreview ?? tooltipSelection);
  let detailPath = $derived(selectionPath(paneSelection));
  let detailKind = $derived.by(() => {
    if (!schema || !paneSelection) return 'empty';
    if (paneSelection.kind === 'record') return 'record';
    if (
      paneSelection.kind === 'resource' ||
      paneSelection.kind === 'resource-record'
    ) return 'resource';
    const entity = schema.entities.find(([, value]) =>
      detailPath && samePath(value.path, detailPath),
    )?.[1];
    if (entity && parseFsm(entity)) return 'fsm';
    if (
      entity &&
      parseResource(entity.annotations)?.kind === 'definition'
    ) return 'resource';
    return entity ? 'events' : 'empty';
  });
  let selectedEntityKind = $derived<'Entity' | 'FSM'>(
    schema &&
      detailPath &&
      schema.entities.some(
        ([, entity]) =>
          samePath(entity.path, detailPath) && Boolean(parseFsm(entity)),
      )
      ? 'FSM'
      : 'Entity',
  );
  let isolateFsmState = $derived(
    selection?.kind === 'fsm-state' &&
      paneSelection?.kind === 'fsm-state' &&
      samePath(selection.entity, paneSelection.entity) &&
      selection.state === paneSelection.state &&
      hoverSelection === null &&
      breadcrumbPreview === null,
  );

  $effect(() => {
    const preference = themePreference;
    saveThemePreference(preference);
    return observeTheme(preference);
  });

  $effect(() => {
    const source = yamlSource;
    let active = true;
    parserStatus = 'parsing';
    parserError = null;
    const timeout = setTimeout(() => {
      void parseYamlSchema(source)
        .then((parsed) => {
          if (!active) return;
          schema = parsed;
          parserStatus = 'valid';
          parserError = null;
          selection = null;
          hoverSelection = null;
          breadcrumbPreview = null;
        })
        .catch((error: unknown) => {
          if (!active) return;
          parserStatus = 'invalid';
          parserError = parserErrorMessage(error);
        });
    }, 180);
    return () => {
      active = false;
      clearTimeout(timeout);
    };
  });

  function selectionPath(value: SchemaSelection | null): SchemaPath | null {
    if (!value) return null;
    if (value.kind === 'record') return value.record;
    if (value.kind === 'resource') return value.resource;
    if (value.kind === 'resource-record') return value.resource;
    if (value.kind === 'reference') return value.reference.source;
    return value.entity;
  }

  function handleSelection(event: CustomEvent<SchemaSelection>): void {
    selectElement(event.detail);
  }

  function handleHover(event: CustomEvent<SchemaSelection>): void {
    hoverSelection = event.detail;
  }

  function handleHoverEnd(): void {
    hoverSelection = null;
  }

  function selectElement(value: SchemaSelection): void {
    selection = value;
    hoverSelection = null;
    breadcrumbPreview = null;
  }

  function previewBreadcrumb(value: SchemaSelection): void {
    breadcrumbPreview = value;
  }

  function endBreadcrumbPreview(): void {
    breadcrumbPreview = null;
  }

  function handleViewChange(
    event: CustomEvent<EntityGraphViewChange>,
  ): void {
    activeView = event.detail.view;
  }

  function setConfig<Key extends keyof ResolvedEntityGraphConfig>(
    key: Key,
    value: ResolvedEntityGraphConfig[Key],
  ): void {
    config = { ...config, [key]: value };
  }

  function toggleGraphExpanded(): void {
    graphExpanded = !graphExpanded;
  }

  function collapseGraphPane(): void {
    graphPane?.collapse();
  }

  function handleWindowKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape' && graphExpanded) {
      graphExpanded = false;
    }
  }

  function changeModel(event: Event): void {
    const nextId = (event.currentTarget as HTMLSelectElement).value;
    const next = yamlExampleModels.find(
      (candidate) => candidate.id === nextId,
    );
    if (!next) return;
    modelId = next.id;
    loadedFileName = null;
    yamlSource = next.source;
    setExampleUrl(next.id);
    selection = null;
    hoverSelection = null;
    breadcrumbPreview = null;
  }

  async function loadYamlFile(event: Event): Promise<void> {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;

    try {
      yamlSource = await file.text();
      loadedFileName = file.name;
      modelId = 'loaded';
      setExampleUrl(null);
      selection = null;
      hoverSelection = null;
      breadcrumbPreview = null;
    } finally {
      input.value = '';
    }
  }

  function setExampleUrl(example: ExampleModelId | null): void {
    const url = new URL(window.location.href);
    if (example) {
      url.searchParams.set('example', example);
    } else {
      url.searchParams.delete('example');
    }
    window.history.replaceState(window.history.state, '', url);
  }

  function downloadYaml(): void {
    const fileName =
      loadedFileName ??
      (modelId === 'loaded' ? 'schema.yaml' : `${modelId}.yaml`);
    const normalizedFileName = /\.ya?ml$/i.test(fileName)
      ? fileName
      : `${fileName}.yaml`;
    const url = URL.createObjectURL(
      new Blob([yamlSource], {
        type: 'application/yaml;charset=utf-8',
      }),
    );
    const link = document.createElement('a');
    link.href = url;
    link.download = normalizedFileName;
    document.body.append(link);
    link.click();
    link.remove();
    URL.revokeObjectURL(url);
  }
</script>

<svelte:head>
  <title>Quent Schema Explorer</title>
</svelte:head>

<svelte:window onkeydown={handleWindowKeydown} />

{#snippet graphWorkspace()}
  <div
    class={[
      'grid h-full min-h-0 min-w-0 grid-rows-[minmax(0,1fr)_auto]',
      !graphExpanded && 'gap-3',
    ]}
  >
    <section
      class="card card-border min-h-0 overflow-hidden bg-base-100"
      aria-label="Schema visualization"
    >
      <div class="schema-explorer-graph min-h-0 h-full overflow-hidden">
        <quent-entity-graph
          class="block h-full min-h-0"
          {schema}
          {selection}
          {config}
          nodeComponent={EntityNode}
          onquent-hover={handleHover}
          onquent-hover-end={handleHoverEnd}
          onquent-select={handleSelection}
          onquent-view-change={handleViewChange}
        ></quent-entity-graph>
      </div>
    </section>

    {#if activeView === 'graph'}
      <GraphConfigBar
        {config}
        expanded={graphExpanded}
        onCollapse={graphExpanded ? undefined : collapseGraphPane}
        onChange={setConfig}
        onToggleExpanded={toggleGraphExpanded}
      />
    {:else}
      <div
        class="card card-border flex-row justify-end gap-2 bg-base-100 p-2"
        aria-label="View pane controls"
      >
        {#if !graphExpanded}
          <button
            class="btn btn-xs btn-ghost"
            type="button"
            onclick={collapseGraphPane}
          >Hide pane</button>
        {/if}
        <button
          class="btn btn-xs"
          type="button"
          aria-pressed={graphExpanded}
          onclick={toggleGraphExpanded}
        >{graphExpanded ? 'Restore' : 'Expand view'}</button>
      </div>
    {/if}
  </div>
{/snippet}

<main
  class={[
    'grid h-dvh min-h-0 overflow-hidden bg-base-200 text-base-content',
    graphExpanded
      ? 'grid-rows-[minmax(0,1fr)]'
      : 'grid-rows-[auto_minmax(0,1fr)] gap-3 p-4',
  ]}
>
  {#if !graphExpanded}
    <header class="flex min-w-0 flex-wrap items-center gap-3">
      <h1 class="truncate text-xl font-semibold">
        Quent Schema Explorer
      </h1>
      <label class="flex shrink-0 items-center gap-2">
        <span class="text-xs font-medium">Example</span>
        <select
          class="select select-bordered select-sm w-48"
          value={modelId}
          onchange={changeModel}
        >
          {#if loadedFileName}
            <option value="loaded">{loadedFileName}</option>
          {/if}
          {#each yamlExampleModels as candidate}
            <option value={candidate.id}>{candidate.label}</option>
          {/each}
        </select>
      </label>
      <input
        class="hidden"
        type="file"
        accept=".yaml,.yml,application/yaml,text/yaml"
        bind:this={yamlFileInput}
        onchange={loadYamlFile}
      />
      <div class="join shrink-0">
        <button
          class="btn btn-sm join-item"
          type="button"
          onclick={() => yamlFileInput?.click()}
        >
          Load
        </button>
        <button
          class="btn btn-sm join-item"
          type="button"
          aria-label="Download current YAML"
          onclick={downloadYaml}
        >
          Save
        </button>
      </div>
      <label class="ml-auto flex shrink-0 items-center gap-2">
        <span class="text-xs font-medium">Theme</span>
        <select
          class="select select-bordered select-sm w-28"
          bind:value={themePreference}
          aria-label="Color theme"
        >
          <option value="system">System</option>
          <option value="light">Light</option>
          <option value="dark">Dark</option>
        </select>
      </label>
    </header>
  {/if}

  {#if graphExpanded}
    {@render graphWorkspace()}
  {:else}
    <PaneGroup
      class="min-h-0 min-w-0"
      direction="horizontal"
      autoSaveId="quent-schema-explorer-main"
    >
      <Pane
        id="yaml-pane"
        bind:this={editorPane}
        class="min-h-0 min-w-0"
        defaultSize={32}
        minSize={15}
        maxSize={45}
        collapsible
        collapsedSize={3}
        onCollapse={() => (editorCollapsed = true)}
        onExpand={() => (editorCollapsed = false)}
      >
        {#if editorCollapsed}
          <button
            class="btn btn-ghost h-full min-h-0 w-full rounded-none px-0"
            type="button"
            aria-label="Expand YAML editor"
            onclick={() => editorPane?.expand()}
          >
            <span class="rotate-180 text-xs [writing-mode:vertical-rl]">YAML</span>
          </button>
        {:else}
          <section
            class="card card-border grid h-full min-h-0 w-full max-w-[80ch] grid-rows-[auto_minmax(0,1fr)_8rem] overflow-hidden bg-base-100"
            aria-label="Quent YAML editor"
          >
            <div class="flex items-center justify-between gap-3 border-b border-base-300 px-3 py-2">
              <strong class="text-sm">Schema YAML</strong>
              <div class="flex items-center gap-2">
                <span
                  class={[
                    'badge badge-sm uppercase',
                    parserStatus === 'valid' && 'badge-success',
                    parserStatus === 'invalid' && 'badge-error',
                    parserStatus === 'parsing' && 'badge-ghost',
                  ].filter(Boolean).join(' ')}
                  data-state={parserStatus}
                >
                  {parserStatus}
                </span>
                <button
                  class="btn btn-xs btn-ghost"
                  type="button"
                  onclick={() => editorPane?.collapse()}
                >Hide</button>
              </div>
            </div>
            <YamlEditor bind:value={yamlSource} />
            <div
              class="grid min-h-0 grid-rows-[auto_minmax(0,1fr)] border-t border-base-300 bg-base-200/50"
              aria-label="YAML errors"
            >
              <div class="flex items-center justify-between px-3 py-1.5">
                <span class="text-xs font-medium">Errors</span>
                {#if parserError}
                  <span class="badge badge-error badge-xs">1</span>
                {/if}
              </div>
              {#if parserError}
                <pre class="min-h-0 overflow-auto bg-error/10 px-3 py-2 font-mono text-xs whitespace-pre-wrap text-error" role="alert">{parserError}</pre>
              {:else}
                <p class="px-3 py-2 text-xs text-base-content/45">
                  {parserStatus === 'parsing' ? 'Checking YAML…' : 'No errors.'}
                </p>
              {/if}
            </div>
          </section>
        {/if}
      </Pane>

      <PaneResizer
        class="group flex w-3 shrink-0 items-center justify-center outline-none"
        aria-label="Resize YAML and explorer panes"
      >
        <span class="pointer-events-none h-12 w-1 rounded-full bg-base-300 group-hover:bg-primary group-focus-visible:bg-primary group-data-[active=pointer]:bg-primary"></span>
      </PaneResizer>

      <Pane
        id="explorer-pane"
        class="min-h-0 min-w-0"
        defaultSize={68}
        minSize={30}
      >
        <PaneGroup
          class="min-h-0 min-w-0"
          direction="horizontal"
          autoSaveId="quent-schema-explorer-details"
        >
          <Pane
            id="graph-pane"
            bind:this={graphPane}
            class="min-h-0 min-w-0"
            defaultSize={65}
            minSize={25}
            collapsible
            collapsedSize={4}
            onCollapse={() => (graphCollapsed = true)}
            onExpand={() => (graphCollapsed = false)}
          >
            {#if graphCollapsed}
              <button
                class="btn btn-ghost h-full min-h-0 w-full rounded-none px-0"
                type="button"
                aria-label="Expand graph pane"
                onclick={() => graphPane?.expand()}
              >
                <span class="rotate-180 text-xs [writing-mode:vertical-rl]">Explorer</span>
              </button>
            {:else}
              {@render graphWorkspace()}
            {/if}
          </Pane>

          <PaneResizer
            class="group flex w-3 shrink-0 items-center justify-center outline-none"
            aria-label="Resize graph and selections panes"
          >
            <span class="pointer-events-none h-12 w-1 rounded-full bg-base-300 group-hover:bg-primary group-focus-visible:bg-primary group-data-[active=pointer]:bg-primary"></span>
          </PaneResizer>

          <Pane
            id="selections-pane"
            bind:this={selectionsPane}
            class="min-h-0 min-w-0"
            defaultSize={35}
            minSize={15}
            collapsible
            collapsedSize={4}
            onCollapse={() => (selectionsCollapsed = true)}
            onExpand={() => (selectionsCollapsed = false)}
          >
            {#if selectionsCollapsed}
              <button
                class="btn btn-ghost h-full min-h-0 w-full rounded-none px-0"
                type="button"
                aria-label="Expand selections pane"
                onclick={() => selectionsPane?.expand()}
              >
                <span class="rotate-180 text-xs [writing-mode:vertical-rl]">Selections</span>
              </button>
            {:else}
              <aside
                class="card card-border grid h-full min-h-0 grid-rows-[auto_auto_minmax(0,1fr)] overflow-hidden bg-base-100"
                aria-label="Selections"
                aria-live="polite"
              >
                <header class="flex items-center justify-between gap-2 border-b border-base-300 px-3 py-2">
                  <h2 class="text-sm font-semibold">Selections</h2>
                  <button
                    class="btn btn-xs btn-ghost"
                    type="button"
                    onclick={() => selectionsPane?.collapse()}
                  >Hide</button>
                </header>
                {#if tooltipSelection}
                  <SelectionBreadcrumbs
                    selection={tooltipSelection}
                    entityKind={selectedEntityKind}
                    onSelect={selectElement}
                    onPreview={previewBreadcrumb}
                    onPreviewEnd={endBreadcrumbPreview}
                  />
                {:else}
                  <div></div>
                {/if}
                <div class="min-h-0 min-w-0 overflow-auto p-3">
                  {#if detailKind === 'fsm'}
                    <quent-fsm-details
                      {schema}
                      path={detailPath}
                      selection={paneSelection}
                      isolateState={isolateFsmState}
                      onquent-select={handleSelection}
                    ></quent-fsm-details>
                  {:else if detailKind === 'resource'}
                    <quent-resource-details
                      {schema}
                      path={detailPath}
                      selection={paneSelection}
                      onquent-select={handleSelection}
                    ></quent-resource-details>
                  {:else if detailKind === 'record'}
                    <quent-record-details
                      {schema}
                      path={detailPath}
                      selection={paneSelection}
                      onquent-select={handleSelection}
                    ></quent-record-details>
                  {:else if detailKind === 'events'}
                    <quent-entity-events
                      {schema}
                      path={detailPath}
                      selection={paneSelection}
                      onquent-select={handleSelection}
                    ></quent-entity-events>
                  {:else}
                    <p class="m-auto max-w-48 text-center text-sm text-base-content/50">
                      Select or hover over an element to inspect it.
                    </p>
                  {/if}
                </div>
              </aside>
            {/if}
          </Pane>
        </PaneGroup>
      </Pane>
    </PaneGroup>
  {/if}
</main>
