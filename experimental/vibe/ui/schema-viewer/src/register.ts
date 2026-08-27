// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import EntityGraph from './components/EntityGraph.svelte';
import EntityEvents from './components/EntityEvents.svelte';
import FsmDetails from './components/FsmDetails.svelte';
import RecordDetails from './components/RecordDetails.svelte';
import ResourceDetails from './components/ResourceDetails.svelte';

interface CompiledCustomElement {
  element: CustomElementConstructor;
}

export function defineSchemaViewerElements(
  registry: CustomElementRegistry | undefined = globalThis.customElements,
): void {
  if (!registry) {
    return;
  }
  defineElement(
    registry,
    'quent-entity-graph',
    EntityGraph as unknown as CompiledCustomElement,
  );
  defineElement(
    registry,
    'quent-entity-events',
    EntityEvents as unknown as CompiledCustomElement,
  );
  defineElement(
    registry,
    'quent-fsm-details',
    FsmDetails as unknown as CompiledCustomElement,
  );
  defineElement(
    registry,
    'quent-record-details',
    RecordDetails as unknown as CompiledCustomElement,
  );
  defineElement(
    registry,
    'quent-resource-details',
    ResourceDetails as unknown as CompiledCustomElement,
  );
}

function defineElement(
  registry: CustomElementRegistry,
  tag: string,
  component: CompiledCustomElement,
): void {
  if (!registry.get(tag)) {
    registry.define(tag, component.element);
  }
}
