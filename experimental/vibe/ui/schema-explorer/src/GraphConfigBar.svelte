<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script lang="ts">
  import type { ResolvedEntityGraphConfig } from '@quent/schema-viewer';

  interface Props {
    config: ResolvedEntityGraphConfig;
    onChange: <Key extends keyof ResolvedEntityGraphConfig>(
      key: Key,
      value: ResolvedEntityGraphConfig[Key],
    ) => void;
  }

  let { config, onChange }: Props = $props();
</script>

<div
  class="card card-border z-20 flex-row flex-wrap items-end gap-2 overflow-visible bg-base-100 p-2"
  data-role="graph-configuration"
  aria-label="Entity graph configuration"
>
  <details class="dropdown dropdown-top order-last ml-auto shrink-0">
    <summary class="btn btn-xs">Layout</summary>
    <div
      class="dropdown-content z-30 grid w-96 max-w-[80vw] grid-cols-2 gap-3 rounded-box border border-base-300 bg-base-100 p-3 shadow-lg"
    >
      <label class="grid gap-1 text-[0.65rem] font-medium">
        Direction
        <select
          class="select select-xs w-full"
          value={config.direction}
          onchange={(event) =>
            onChange(
              'direction',
              event.currentTarget
                .value as ResolvedEntityGraphConfig['direction'],
            )}
        >
          <option value="down">Down</option>
          <option value="right">Right</option>
          <option value="up">Up</option>
          <option value="left">Left</option>
        </select>
      </label>
      <label class="grid gap-1 text-[0.65rem] font-medium">
        Routing
        <select
          class="select select-xs w-full"
          value={config.edgeRouting}
          onchange={(event) =>
            onChange(
              'edgeRouting',
              event.currentTarget
                .value as ResolvedEntityGraphConfig['edgeRouting'],
            )}
        >
          <option value="polyline">Polyline</option>
          <option value="orthogonal">Orthogonal</option>
        </select>
      </label>
      <label class="grid gap-1 text-[0.65rem] font-medium">
        Density
        <select
          class="select select-xs w-full"
          value={config.density}
          onchange={(event) =>
            onChange(
              'density',
              event.currentTarget
                .value as ResolvedEntityGraphConfig['density'],
            )}
        >
          <option value="compact">Compact</option>
          <option value="comfortable">Comfortable</option>
          <option value="spacious">Spacious</option>
        </select>
      </label>
      <label class="grid gap-1 text-[0.65rem] font-medium">
        Layering
        <select
          class="select select-xs w-full"
          value={config.layeringStrategy}
          onchange={(event) =>
            onChange(
              'layeringStrategy',
              event.currentTarget
                .value as ResolvedEntityGraphConfig['layeringStrategy'],
            )}
        >
          <option value="network-simplex">Network simplex</option>
          <option value="longest-path">Longest path</option>
          <option value="longest-path-source">Longest path from sources</option>
          <option value="coffman-graham">Coffman–Graham</option>
        </select>
      </label>
      <label class="grid gap-1 text-[0.65rem] font-medium">
        Node placement
        <select
          class="select select-xs w-full"
          value={config.nodePlacementStrategy}
          onchange={(event) =>
            onChange(
              'nodePlacementStrategy',
              event.currentTarget
                .value as ResolvedEntityGraphConfig['nodePlacementStrategy'],
            )}
        >
          <option value="brandes-koepf">Brandes–Köpf</option>
          <option value="network-simplex">Network simplex</option>
          <option value="linear-segments">Linear segments</option>
          <option value="simple">Simple</option>
        </select>
      </label>
      <label class="grid gap-1 text-[0.65rem] font-medium">
        Layout effort
        <select
          class="select select-xs w-full"
          value={config.layoutThoroughness}
          onchange={(event) =>
            onChange(
              'layoutThoroughness',
              Number(event.currentTarget.value),
            )}
        >
          <option value="7">Normal</option>
          <option value="20">Thorough</option>
          <option value="50">Maximum</option>
        </select>
      </label>
      <label class="label cursor-pointer justify-start gap-2 py-0 text-xs">
        <input
          class="toggle toggle-xs"
          type="checkbox"
          checked={config.hierarchicalGreedySwitch}
          onchange={(event) =>
            onChange(
              'hierarchicalGreedySwitch',
              event.currentTarget.checked,
            )}
        />
        Reduce hierarchy crossings
      </label>
      <label class="label col-span-2 cursor-pointer justify-start gap-2 py-0 text-xs">
        <input
          class="toggle toggle-xs"
          type="checkbox"
          checked={config.highDegreeNodeTreatment}
          onchange={(event) =>
            onChange(
              'highDegreeNodeTreatment',
              event.currentTarget.checked,
            )}
        />
        Make room around high-degree nodes
      </label>
    </div>
  </details>
  <fieldset class="grid shrink-0 gap-1">
    <legend class="text-[0.65rem] font-medium">References</legend>
    <div class="join" aria-label="Reference filter">
      <button
        type="button"
        class={[
          'btn btn-xs join-item',
          config.references === 'all' ? 'btn-primary' : 'btn-ghost',
        ]}
        aria-pressed={config.references === 'all'}
        onclick={() => onChange('references', 'all')}
      >All</button>
      <button
        type="button"
        class={[
          'btn btn-xs join-item',
          config.references === 'untyped' ? 'btn-primary' : 'btn-ghost',
        ]}
        aria-pressed={config.references === 'untyped'}
        onclick={() => onChange('references', 'untyped')}
      >Untyped</button>
      <button
        type="button"
        class={[
          'btn btn-xs join-item',
          config.references === 'typed' ? 'btn-primary' : 'btn-ghost',
        ]}
        aria-pressed={config.references === 'typed'}
        onclick={() => onChange('references', 'typed')}
      >Typed</button>
      <button
        type="button"
        class={[
          'btn btn-xs join-item',
          config.references === 'tree' ? 'btn-primary' : 'btn-ghost',
        ]}
        aria-pressed={config.references === 'tree'}
        onclick={() => onChange('references', 'tree')}
      >Tree</button>
    </div>
  </fieldset>
  <fieldset class="grid shrink-0 gap-1">
    <legend class="text-[0.65rem] font-medium">Edge labels</legend>
    <div class="join" aria-label="Reference label visibility">
      <button
        type="button"
        class={[
          'btn btn-xs join-item',
          config.referenceLabels === 'interaction'
            ? 'btn-primary'
            : 'btn-ghost',
        ]}
        aria-pressed={config.referenceLabels === 'interaction'}
        onclick={() => onChange('referenceLabels', 'interaction')}
      >On hover</button>
      <button
        type="button"
        class={[
          'btn btn-xs join-item',
          config.referenceLabels === 'always'
            ? 'btn-primary'
            : 'btn-ghost',
        ]}
        aria-pressed={config.referenceLabels === 'always'}
        onclick={() => onChange('referenceLabels', 'always')}
      >Always</button>
      <button
        type="button"
        class={[
          'btn btn-xs join-item',
          config.referenceLabels === 'never'
            ? 'btn-primary'
            : 'btn-ghost',
        ]}
        aria-pressed={config.referenceLabels === 'never'}
        onclick={() => onChange('referenceLabels', 'never')}
      >Never</button>
    </div>
  </fieldset>
  <fieldset class="grid shrink-0 gap-1">
    <legend class="text-[0.65rem] font-medium">Namespaces</legend>
    <div class="join" aria-label="Namespace grouping">
      <button
        type="button"
        class={[
          'btn btn-xs join-item',
          config.groupNamespaces ? 'btn-primary' : 'btn-ghost',
        ]}
        aria-pressed={config.groupNamespaces}
        onclick={() => onChange('groupNamespaces', true)}
      >Show</button>
      <button
        type="button"
        class={[
          'btn btn-xs join-item',
          !config.groupNamespaces ? 'btn-primary' : 'btn-ghost',
        ]}
        aria-pressed={!config.groupNamespaces}
        onclick={() => onChange('groupNamespaces', false)}
      >Hide</button>
    </div>
  </fieldset>
  <fieldset class="grid shrink-0 gap-1">
    <legend class="text-[0.65rem] font-medium">Edges</legend>
    <div
      class="flex h-6 items-center gap-3 px-1 text-[0.65rem]"
      aria-label="Edge legend"
    >
      <span class="flex items-center gap-1">
        <svg
          class="h-3 w-8"
          viewBox="0 0 32 12"
          aria-hidden="true"
        >
          <path
            d="M 1 6 H 25"
            fill="none"
            stroke="var(--quent-viewer-muted)"
            stroke-width="2"
          />
          <path
            d="M 25 2 L 31 6 L 25 10 Z"
            fill="var(--quent-viewer-muted)"
          />
        </svg>
        <span>Reference</span>
      </span>
      <span class="flex items-center gap-1">
        <svg
          class="h-3 w-8"
          viewBox="0 0 32 12"
          aria-hidden="true"
        >
          <path
            d="M 1 6 H 25"
            fill="none"
            stroke="var(--quent-viewer-tree)"
            stroke-width="4"
          />
          <path
            d="M 25 1 L 31 6 L 25 11 Z"
            fill="var(--quent-viewer-tree)"
          />
        </svg>
        <span>Tree-forming</span>
      </span>
    </div>
  </fieldset>
</div>
