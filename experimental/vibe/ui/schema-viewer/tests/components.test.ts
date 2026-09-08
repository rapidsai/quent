// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { tick } from 'svelte';
import { afterEach, describe, expect, test, vi } from 'vitest';

import { sampleSchema } from './fixtures/sample-schema';
import TestEntityNode from './fixtures/TestEntityNode.svelte';
import '../src/index';
import type {
  QuentEntityGraphElement,
  QuentFsmDetailsElement,
  QuentPathDetailsElement,
  SchemaPath,
} from '../src/public';

function path(value: string): SchemaPath {
  const segments = value.split('::');
  return {
    namespace: segments.slice(0, -1),
    name: segments.at(-1)!,
  };
}

async function renderGraph(
  configure?: (graph: QuentEntityGraphElement) => void,
): Promise<QuentEntityGraphElement> {
  const graph = document.createElement(
    'quent-entity-graph',
  ) as QuentEntityGraphElement;
  configure?.(graph);
  document.body.append(graph);
  graph.schema = sampleSchema;
  await vi.waitFor(() => {
    expect(
      graph.querySelectorAll('[data-quent-role="entity"]'),
    ).toHaveLength(26);
  });
  return graph;
}

afterEach(() => {
  document.body.replaceChildren();
});

describe('schema viewer elements', () => {
  test('renders XYFlow entities, namespace parents, and type distinctions', async () => {
    const graph = await renderGraph();

    expect(
      graph.querySelectorAll('[data-quent-role="namespace"]').length,
    ).toBeGreaterThan(0);
    expect(
      graph.querySelectorAll('[data-quent-role="entity"][data-fsm="true"]'),
    ).toHaveLength(9);
    expect(
      graph.querySelectorAll(
        '[data-quent-role="entity"][data-resource="true"]',
      ),
    ).toHaveLength(6);
    expect(graph.querySelector('.svelte-flow')).not.toBeNull();
    expect(
      graph.querySelectorAll(
        '[data-quent-role="entity-title"]' +
          '[data-quent-schema-name="true"]',
      ),
    ).toHaveLength(26);
    expect(
      graph.querySelector(
        '[data-quent-role="namespace"] [data-quent-schema-name="true"]',
      ),
    ).not.toBeNull();
    expect(graph.querySelector('.svelte-flow__background')).toBeNull();
    expect(graph.querySelector('.svelte-flow__attribution')).toBeNull();
    expect(
      graph.querySelectorAll('.svelte-flow__handle').length,
    ).toBeGreaterThan(0);
    expect(
      graph.querySelectorAll('.quent-entity-flow__handle').length,
    ).toBeGreaterThan(0);
  });

  test('emits typed select and hover callbacks from XYFlow nodes', async () => {
    const onSelect = vi.fn();
    const onHover = vi.fn();
    const onHoverEnd = vi.fn();
    const graph = await renderGraph((element) => {
      element.addEventListener('quent-select', onSelect);
      element.addEventListener('quent-hover', onHover);
      element.addEventListener('quent-hover-end', onHoverEnd);
    });
    const worker = graph.querySelector<HTMLElement>(
      '[data-entity="Service::Worker"]',
    )!;

    worker.dispatchEvent(new PointerEvent('pointerenter', { bubbles: true }));
    worker.dispatchEvent(new MouseEvent('click', { bubbles: true }));

    await vi.waitFor(() => expect(onSelect).toHaveBeenCalledOnce());
    expect(onSelect.mock.calls[0][0].detail).toEqual({
      kind: 'entity',
      entity: path('Service::Worker'),
    });
    expect(onHover).toHaveBeenCalledOnce();
    expect(onHover.mock.calls[0][0].detail).toEqual({
      kind: 'entity',
      entity: path('Service::Worker'),
    });
    worker.dispatchEvent(new PointerEvent('pointerleave', { bubbles: true }));
    expect(onHoverEnd).toHaveBeenCalledOnce();
  });

  test('renders a supplied entity node component', async () => {
    const graph = await renderGraph((element) => {
      element.nodeComponent = TestEntityNode;
    });

    expect(
      graph.querySelector('[data-entity="Service::Worker"] [data-test-node]')
        ?.textContent,
    ).toBe('SchemaViewerStress:Service/Worker');
    expect(
      graph.querySelector(
        '[data-entity="Service::Worker"] ' +
          '[data-quent-role="entity-type-badge"]',
      )?.textContent,
    ).toContain('FSM');
    expect(
      graph.querySelector(
        '[data-entity="Resource::Memory"] ' +
          '[data-quent-role="entity-type-badge"]',
      )?.textContent,
    ).toContain('Resource');
  });

  test('switches immediately to the static timeline and selects FSM states', async () => {
    const onViewChange = vi.fn();
    const onSelect = vi.fn();
    const onHover = vi.fn();
    const onHoverEnd = vi.fn();
    const graph = await renderGraph((element) => {
      element.addEventListener('quent-view-change', onViewChange);
      element.addEventListener('quent-select', onSelect);
      element.addEventListener('quent-hover', onHover);
      element.addEventListener('quent-hover-end', onHoverEnd);
    });

    graph
      .querySelector<HTMLButtonElement>(
        '[data-quent-role="view-resource-timeline"]',
      )!
      .click();
    await tick();

    expect(
      graph.querySelector('[data-quent-role="resource-timeline"]'),
    ).not.toBeNull();
    expect(
      graph.querySelector<HTMLElement>(
        '[data-quent-view-panel="graph"]',
      )?.hidden,
    ).toBe(true);
    expect(
      graph.querySelector<HTMLElement>(
        '[data-quent-view-panel="resource-timeline"]',
      )?.hidden,
    ).toBe(false);
    expect(
      graph.querySelectorAll('[data-quent-role="timeline-row"]'),
    ).toHaveLength(26);
    expect(
      graph.querySelectorAll('[data-quent-role="timeline-entity-badge"]'),
    ).toHaveLength(26);
    expect(
      graph.querySelectorAll(
        '[data-quent-role="timeline-entity-title"]' +
          '[data-quent-schema-name="true"]',
      ),
    ).toHaveLength(26);
    expect(
      graph.querySelector(
        '[data-quent-role="timeline-fsm-state"] ' +
          '[data-quent-schema-name="true"]',
      ),
    ).not.toBeNull();
    expect(
      new Set(
        Array.from(
          graph.querySelectorAll(
            '[data-quent-role="timeline-entity-badge"]',
          ),
          (badge) => badge.textContent?.trim(),
        ),
      ),
    ).toEqual(new Set(['Entity', 'FSM', 'Resource']));
    expect(
      graph.querySelectorAll(
        '[data-quent-role="timeline-row"][data-resource-in-scope="false"]',
      ).length,
    ).toBeGreaterThan(0);
    expect(
      graph.querySelector('[data-quent-role="timeline-filtered-heading"]')
        ?.textContent,
    ).toContain('No resources in scope');
    expect(
      graph.querySelectorAll(
        '[data-quent-role="timeline-fsm-state"][data-uses-resource="true"]',
      ).length,
    ).toBeGreaterThan(0);
    expect(
      graph.querySelectorAll('[data-quent-role="timeline-capacity-bin"]')
        .length,
    ).toBeGreaterThan(0);
    expect(
      graph.querySelector('[data-quent-role="timeline-usage-stream"]'),
    ).toBeNull();
    expect(
      graph.querySelector('[data-quent-role="timeline-usage-impact"]'),
    ).toBeNull();

    const fsm = graph.querySelector<HTMLButtonElement>(
      '[data-quent-role="timeline-fsm-select"]',
    )!;
    fsm.click();
    expect(onSelect.mock.calls.at(-1)?.[0].detail).toMatchObject({
      kind: 'entity',
      entity: { name: fsm.textContent?.trim() },
    });
    const state = graph.querySelector<HTMLButtonElement>(
      '[data-quent-role="timeline-fsm-state"]',
    )!;
    state.dispatchEvent(new PointerEvent('pointerenter', { bubbles: true }));
    expect(onHover.mock.calls.at(-1)?.[0].detail).toMatchObject({
      kind: 'fsm-state',
      state: state.dataset.state,
    });
    state.dispatchEvent(new PointerEvent('pointerleave', { bubbles: true }));
    expect(onHoverEnd).toHaveBeenCalledOnce();
    state.click();
    expect(onSelect.mock.calls.at(-1)?.[0].detail).toMatchObject({
      kind: 'fsm-state',
      state: state.dataset.state,
    });
    expect(onViewChange).toHaveBeenCalledWith(
      expect.objectContaining({
        detail: { view: 'resource-timeline' },
      }),
    );

    graph
      .querySelector<HTMLButtonElement>('[data-quent-role="view-graph"]')!
      .click();
    await tick();
    expect(
      graph.querySelector<HTMLElement>(
        '[data-quent-view-panel="graph"]',
      )?.hidden,
    ).toBe(false);
  });

  test('applies graph configuration and emits layout lifecycle events', async () => {
    const onStart = vi.fn();
    const onComplete = vi.fn();
    const graph = await renderGraph((element) => {
      element.config = {
        groupNamespaces: false,
        references: 'tree',
        showViewSwitcher: false,
        showNodeMetadata: false,
      };
      element.addEventListener('quent-layout-start', onStart);
      element.addEventListener('quent-layout-complete', onComplete);
    });

    expect(onStart).toHaveBeenCalledOnce();
    expect(onComplete).toHaveBeenCalledOnce();
    expect(onComplete.mock.calls[0][0].detail).toMatchObject({
      nodeCount: 26,
      referenceCount: 26,
    });
    expect(
      graph.querySelector('[data-quent-role="layout-status"]')?.textContent,
    ).toContain('26 nodes · 26 references ·');
    expect(graph.querySelector('[data-quent-role="namespace"]')).toBeNull();
    expect(graph.querySelector('[data-quent-role="view-switcher"]')).toBeNull();
    expect(graph.querySelector('[data-quent-role="badges"]')).not.toBeNull();
    expect(
      graph.querySelector('[data-quent-role="viewport-controls"]'),
    ).not.toBeNull();
    expect(graph.querySelector('.quent-entity-graph__node-meta')).toBeNull();
  });

  test('places working viewport controls below the graph', async () => {
    const graph = await renderGraph();
    const controls = graph.querySelector(
      '[data-quent-role="viewport-controls"]',
    );

    expect(controls).not.toBeNull();
    expect(
      controls?.parentElement?.classList.contains('quent-entity-flow'),
    ).toBe(true);
    expect(controls?.closest('.svelte-flow')).toBeNull();
    expect(
      Array.from(
        controls?.querySelectorAll<HTMLButtonElement>('button') ?? [],
      ),
    ).toHaveLength(3);
    expect(
      Array.from(
        controls?.querySelectorAll<HTMLButtonElement>('button') ?? [],
      ).every((button) => !button.disabled),
    ).toBe(true);
  });

  test('requests fit-view after schema changes but not layout tuning', async () => {
    const graph = await renderGraph();
    const initialLayoutVersion = Number(
      graph
        .querySelector('[data-quent-role="viewport"]')
        ?.getAttribute('data-layout-version'),
    );
    const initialFitVersion = Number(
      graph
        .querySelector('[data-quent-role="viewport"]')
        ?.getAttribute('data-fit-version'),
    );

    graph.config = { direction: 'right' };
    await vi.waitFor(() => {
      expect(
        Number(
          graph
            .querySelector('[data-quent-role="viewport"]')
            ?.getAttribute('data-layout-version'),
        ),
      ).toBeGreaterThan(initialLayoutVersion);
    });
    expect(
      Number(
        graph
          .querySelector('[data-quent-role="viewport"]')
          ?.getAttribute('data-fit-version'),
      ),
    ).toBe(initialFitVersion);

    graph.schema = {
      ...sampleSchema,
      name: 'ReplacementSchema',
    };

    await vi.waitFor(() => {
      expect(
        Number(
          graph
            .querySelector('[data-quent-role="viewport"]')
            ?.getAttribute('data-layout-version'),
        ),
      ).toBeGreaterThan(initialLayoutVersion + 1);
      expect(
        Number(
          graph
            .querySelector('[data-quent-role="viewport"]')
            ?.getAttribute('data-fit-version'),
        ),
      ).toBeGreaterThan(initialFitVersion);
    });
  });

  test('shows focused FSM and entity event details', async () => {
    const details = document.createElement(
      'quent-fsm-details',
    ) as QuentFsmDetailsElement;
    details.schema = sampleSchema;
    details.path = path('Service::Worker');
    details.selection = {
      kind: 'fsm-state',
      entity: path('Service::Worker'),
      state: 'busy',
    };
    document.body.append(details);

    await vi.waitFor(() => {
      expect(
        details.querySelector('[data-quent-role="fsm-graph"] .svelte-flow'),
      ).not.toBeNull();
    });
    expect(
      details.querySelector('[data-quent-role="fsm-exit"]'),
    ).not.toBeNull();
    expect(
      details.querySelector(
        '[data-quent-role="fsm-state-title"][data-quent-schema-name="true"]',
      ),
    ).not.toBeNull();
    expect(
      details.querySelector('.svelte-flow__attribution'),
    ).toBeNull();
    expect(
      details.querySelector(
        '[data-quent-role="fsm-state"][data-state="busy"]' +
          '.quent-schema-details__fsm-flow-node-wrapper--selected',
      ),
    ).not.toBeNull();
    const fsmTitle = details.querySelector('[data-quent-role="fsm-title"]')!;
    const fsmBadge = details.querySelector('[data-quent-role="fsm-badge"]')!;
    expect(
      fsmTitle.compareDocumentPosition(fsmBadge) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();

    details.isolateState = true;
    await vi.waitFor(() => {
      expect(
        details.querySelector(
          '[data-quent-role="fsm-attributes"][data-state="busy"]' +
            '[data-selected="true"]',
        ),
      ).not.toBeNull();
    });
    expect(
      details.querySelector('[data-quent-role="fsm-graph"]'),
    ).toBeNull();
    expect(details.textContent).toContain('ResourceData::CpuUsage');
    const stateTitle = details.querySelector(
      '[data-quent-role="fsm-state-title"]',
    )!;
    expect(stateTitle.getAttribute('data-quent-schema-name')).toBe('true');
    const stateBadge = stateTitle.nextElementSibling!;
    expect(
      stateTitle.compareDocumentPosition(stateBadge) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(stateBadge.textContent).toContain('State');

    const events = document.createElement(
      'quent-entity-events',
    ) as QuentPathDetailsElement;
    events.schema = sampleSchema;
    events.path = path('Platform');
    document.body.append(events);
    await vi.waitFor(() => {
      expect(events.querySelector('[data-quent-role="events"]')).not.toBeNull();
    });
    const eventTitle = events.querySelector('[data-quent-role="event-title"]')!;
    expect(eventTitle.getAttribute('data-quent-schema-name')).toBe('true');
    const eventBadge = events.querySelector('[data-quent-role="event-badge"]')!;
    expect(
      eventTitle.compareDocumentPosition(eventBadge) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });
});
