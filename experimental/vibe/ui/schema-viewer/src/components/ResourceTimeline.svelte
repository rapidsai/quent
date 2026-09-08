<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script lang="ts">
  import type { Schema } from '@quent/schema';

  import { buildResourceTimeline } from '../lib/resourceTimeline';
  import { selectionMatches } from '../lib/selection';
  import type {
    EntityGraphClasses,
    EntityGraphModel,
    SchemaSelection,
  } from '../lib/types';

  interface Props {
    schema: Schema;
    model: EntityGraphModel;
    selection: SchemaSelection | null;
    classes: EntityGraphClasses;
    onSelect: (selection: SchemaSelection) => void;
    onHover: (selection: SchemaSelection) => void;
    onHoverEnd: () => void;
  }

  let {
    schema,
    model,
    selection,
    classes,
    onSelect,
    onHover,
    onHoverEnd,
  }: Props = $props();
  let timeline = $derived(buildResourceTimeline(model, schema));
  let resourceRows = $derived(
    timeline.rows.filter((row) => row.resourceInScope),
  );
  let filteredRows = $derived(
    timeline.rows.filter((row) => !row.resourceInScope),
  );
</script>

<div
  class={`quent-resource-timeline ${classes.timeline ?? ''}`}
  data-quent-role="resource-timeline"
  data-quent-view="resource-timeline"
>
  <header
    class={`quent-resource-timeline__header ${classes.timelineHeader ?? ''}`}
  >
    <div>
      <strong>Resource utilization</strong>
      <span>Static schema-derived illustration</span>
    </div>
    <p>
      Capacity bins aggregate all FSM states that use each resource. They do
      not represent runtime measurements.
    </p>
  </header>

  {#if !timeline.hasResources}
    <div class="quent-schema-viewer__empty">
      No resource definitions in this schema.
    </div>
  {:else}
    <div class="quent-resource-timeline__rows">
      {#each resourceRows as row (row.node.id)}
        <article
          class={[
            'quent-resource-timeline__row',
            row.node.resource && 'quent-resource-timeline__row--resource',
            classes.timelineRow,
          ].filter(Boolean).join(' ')}
          data-quent-role="timeline-row"
          data-resource-in-scope="true"
        >
          <div
            class="quent-resource-timeline__entity"
            style={`--quent-tree-depth:${Math.min(row.depth, 8)}`}
          >
            <span class="quent-resource-timeline__tree-mark" aria-hidden="true">
              {row.depth === 0 ? '●' : '└'}
            </span>
            <span class="quent-resource-timeline__entity-copy">
              {#if row.node.path.namespace.length > 0}
                <small
                  class="quent-resource-timeline__namespace quent-schema-name"
                  data-quent-role="timeline-namespace"
                  data-quent-schema-name="true"
                  data-namespace={row.node.path.namespace.join('::')}
                >
                  {row.node.path.namespace.join(' / ')}
                </small>
              {/if}
              <span class="quent-resource-timeline__entity-title">
                <strong
                  class="quent-schema-name quent-schema-name--title"
                  data-quent-role="timeline-entity-title"
                  data-quent-schema-name="true"
                >
                  {row.node.path.name}
                </strong>
                <span
                  class={[
                    'quent-schema-viewer__badge',
                    row.node.resource
                      ? 'quent-schema-viewer__badge--resource'
                      : row.node.fsm
                        ? 'quent-schema-viewer__badge--fsm'
                        : 'quent-schema-viewer__badge--entity',
                  ].join(' ')}
                  data-quent-role="timeline-entity-badge"
                >
                  {row.node.resource
                    ? 'Resource'
                    : row.node.fsm
                      ? 'FSM'
                      : 'Entity'}
                </span>
              </span>
            </span>
          </div>

          <div
            class={`quent-resource-timeline__track ${classes.timelineTrack ?? ''}`}
          >
            {#if row.node.resource}
              {#if row.sequences.length === 0}
                <span class="quent-resource-timeline__empty">
                  No FSM state uses this resource.
                </span>
              {:else}
                <div class="quent-resource-timeline__sequences">
                  {#each row.sequences as sequence (sequence.id)}
                    <section
                      class="quent-resource-timeline__fsm"
                      data-quent-role="timeline-fsm"
                    >
                      <button
                        type="button"
                        class="quent-resource-timeline__fsm-label"
                        data-quent-role="timeline-fsm-select"
                        onclick={() =>
                          onSelect({
                            kind: 'entity',
                            entity: sequence.entity,
                          })}
                      >
                        <span
                          class="quent-schema-name"
                          data-quent-schema-name="true"
                        >
                          {sequence.label}
                        </span>
                      </button>
                      <div class="quent-resource-timeline__states">
                        {#each sequence.states as state (state.id)}
                          {@const stateSelection = {
                            kind: 'fsm-state',
                            entity: sequence.entity,
                            state: state.name,
                          } as const}
                          <button
                            type="button"
                            class={[
                              'quent-resource-timeline__state',
                              state.usesResource &&
                                'quent-resource-timeline__state--uses-resource',
                              selectionMatches(selection, stateSelection) &&
                                'quent-resource-timeline__state--selected',
                              classes.timelineSegment,
                            ].filter(Boolean).join(' ')}
                            data-quent-role="timeline-fsm-state"
                            data-state={state.name}
                            data-uses-resource={state.usesResource}
                            style={`width:${state.width}%`}
                            title={state.capacities.length > 0
                              ? `${state.name}: ${state.capacities.join(', ')}`
                              : state.name}
                            onpointerenter={() => onHover(stateSelection)}
                            onpointerleave={onHoverEnd}
                            onclick={() => onSelect(stateSelection)}
                          >
                            <span
                              class="quent-schema-name"
                              data-quent-schema-name="true"
                            >
                              {state.name}
                            </span>
                          </button>
                        {/each}
                      </div>
                    </section>
                  {/each}
                </div>
              {/if}

              <div class="quent-resource-timeline__capacities">
                {#each row.capacities as capacity (capacity.id)}
                  <div
                    class="quent-resource-timeline__capacity"
                    data-quent-role="timeline-capacity"
                  >
                    <span>
                      <strong
                        class="quent-schema-name"
                        data-quent-schema-name="true"
                      >
                        {capacity.name}
                      </strong>
                      <small>
                        {capacity.kind}
                        {capacity.bounded ? ' · bounded' : ''}
                      </small>
                    </span>
                    <div
                      class="quent-resource-timeline__bins"
                      aria-label={`${capacity.name} illustrative utilization`}
                    >
                      {#each capacity.bins as bin (bin.id)}
                        <i
                          data-quent-role="timeline-capacity-bin"
                          style={`height:${bin.height}px`}
                        ></i>
                      {/each}
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        </article>
      {/each}
    </div>

    {#if filteredRows.length > 0}
      <section
        class="quent-resource-timeline__filtered"
        data-quent-role="timeline-filtered-heading"
      >
        <header>
          <strong>No resources in scope</strong>
          <span>{filteredRows.length} schema entities</span>
        </header>
        {#each filteredRows as row (row.node.id)}
          <div
            class={`quent-resource-timeline__row quent-resource-timeline__row--filtered ${classes.timelineRow ?? ''}`}
            data-quent-role="timeline-row"
            data-resource-in-scope="false"
          >
            <div
              class="quent-resource-timeline__entity"
              style={`--quent-tree-depth:${Math.min(row.depth, 8)}`}
            >
              <span class="quent-resource-timeline__tree-mark" aria-hidden="true">
                {row.depth === 0 ? '○' : '└'}
              </span>
              <span class="quent-resource-timeline__entity-copy">
                {#if row.node.path.namespace.length > 0}
                  <small
                    class="quent-resource-timeline__namespace quent-schema-name"
                    data-quent-role="timeline-namespace"
                    data-quent-schema-name="true"
                    data-namespace={row.node.path.namespace.join('::')}
                  >
                    {row.node.path.namespace.join(' / ')}
                  </small>
                {/if}
                <span class="quent-resource-timeline__entity-title">
                  <strong
                    class="quent-schema-name quent-schema-name--title"
                    data-quent-role="timeline-entity-title"
                    data-quent-schema-name="true"
                  >
                    {row.node.path.name}
                  </strong>
                  <span
                    class={[
                      'quent-schema-viewer__badge',
                      row.node.resource
                        ? 'quent-schema-viewer__badge--resource'
                        : row.node.fsm
                          ? 'quent-schema-viewer__badge--fsm'
                          : 'quent-schema-viewer__badge--entity',
                    ].join(' ')}
                    data-quent-role="timeline-entity-badge"
                  >
                    {row.node.resource
                      ? 'Resource'
                      : row.node.fsm
                        ? 'FSM'
                        : 'Entity'}
                  </span>
                </span>
              </span>
            </div>
          </div>
        {/each}
      </section>
    {/if}
  {/if}
</div>
