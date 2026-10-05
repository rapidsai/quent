// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { PropsWithChildren } from 'react';
import { render, renderHook } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { COLOR_PALETTES, createColorRegistry, getDeterministicColor } from '@quent/utils';
import {
  COLOR_REGISTRY_KEYS,
  ColorRegistryProvider,
  useColorResolver,
  type ColorRegistry,
} from './colorRegistry';

function registryValue(entries: Iterable<readonly [string, string]>) {
  return {
    colorMap: new Map(entries),
    palette: COLOR_PALETTES.deterministic,
  };
}

function createWrapper(registry: ColorRegistry) {
  return function Wrapper({ children }: PropsWithChildren) {
    return <ColorRegistryProvider registry={registry}>{children}</ColorRegistryProvider>;
  };
}

describe('color registry', () => {
  it('hydrates namespace resolvers before their first read', () => {
    const registry: ColorRegistry = new Map([
      [COLOR_REGISTRY_KEYS.OPERATOR_TYPES, registryValue([['project', '#123456']])],
    ]);
    const { result } = renderHook(() => useColorResolver(COLOR_REGISTRY_KEYS.OPERATOR_TYPES), {
      wrapper: createWrapper(registry),
    });

    expect(result.current('Project')).toBe('#123456');
  });

  it('updates descendants with a new registry in the same render', () => {
    const first: ColorRegistry = new Map([
      [COLOR_REGISTRY_KEYS.OPERATOR_TYPES, registryValue([['project', '#111111']])],
    ]);
    const second: ColorRegistry = new Map([
      [COLOR_REGISTRY_KEYS.OPERATOR_TYPES, registryValue([['project', '#222222']])],
    ]);
    const onRender = vi.fn();

    function Probe() {
      const resolveColor = useColorResolver(COLOR_REGISTRY_KEYS.OPERATOR_TYPES);
      onRender(resolveColor('project'));
      return null;
    }

    const { rerender } = render(
      <ColorRegistryProvider registry={first}>
        <Probe />
      </ColorRegistryProvider>
    );
    onRender.mockClear();

    rerender(
      <ColorRegistryProvider registry={second}>
        <Probe />
      </ColorRegistryProvider>
    );

    expect(onRender).toHaveBeenCalled();
    expect(onRender.mock.calls.every(([color]) => color === '#222222')).toBe(true);
  });

  it('keeps assignments independent across registry keys', () => {
    const registry: ColorRegistry = new Map([
      [COLOR_REGISTRY_KEYS.OPERATOR_TYPES, registryValue([['shared', '#111111']])],
      [COLOR_REGISTRY_KEYS.RESOURCE_TYPES, registryValue([['shared', '#222222']])],
    ]);
    const { result } = renderHook(
      () => ({
        operator: useColorResolver(COLOR_REGISTRY_KEYS.OPERATOR_TYPES),
        resource: useColorResolver(COLOR_REGISTRY_KEYS.RESOURCE_TYPES),
      }),
      { wrapper: createWrapper(registry) }
    );

    expect(result.current.operator('shared')).toBe('#111111');
    expect(result.current.resource('shared')).toBe('#222222');
  });

  it('isolates registries in separate query providers', () => {
    const first: ColorRegistry = new Map([
      [COLOR_REGISTRY_KEYS.OPERATOR_TYPES, registryValue([['project', '#111111']])],
    ]);
    const second: ColorRegistry = new Map([
      [COLOR_REGISTRY_KEYS.OPERATOR_TYPES, registryValue([['project', '#222222']])],
    ]);
    const firstResult = renderHook(() => useColorResolver(COLOR_REGISTRY_KEYS.OPERATOR_TYPES), {
      wrapper: createWrapper(first),
    });
    const secondResult = renderHook(() => useColorResolver(COLOR_REGISTRY_KEYS.OPERATOR_TYPES), {
      wrapper: createWrapper(second),
    });

    expect(firstResult.result.current('project')).toBe('#111111');
    expect(secondResult.result.current('project')).toBe('#222222');
  });

  it('falls back when a namespace or key was not hydrated', () => {
    const { result } = renderHook(() => useColorResolver(COLOR_REGISTRY_KEYS.FSM_TYPES), {
      wrapper: createWrapper(new Map()),
    });

    expect(result.current('Task')).toBe(getDeterministicColor('Task'));
  });

  it('wraps runtime values in a collision-aware resolver', () => {
    const registry: ColorRegistry = new Map([
      [COLOR_REGISTRY_KEYS.DATA_FLOW_DIMENSIONS, registryValue([['declared', '#3b82f6']])],
    ]);
    const { result } = renderHook(
      () => useColorResolver(COLOR_REGISTRY_KEYS.DATA_FLOW_DIMENSIONS, ['synthetic']),
      { wrapper: createWrapper(registry) }
    );

    expect(result.current('declared')).toBe('#3b82f6');
    expect(result.current('synthetic')).not.toBe('#3b82f6');
  });

  it('shares incremental assignments for omitted namespaces with theme palettes', () => {
    const registry = createColorRegistry([], 'dark');
    const { result } = renderHook(
      () => ({
        first: useColorResolver(COLOR_REGISTRY_KEYS.DATA_FLOW_DIMENSIONS),
        second: useColorResolver(COLOR_REGISTRY_KEYS.DATA_FLOW_DIMENSIONS),
      }),
      { wrapper: createWrapper(registry) }
    );

    // These keys hash to the same slot in the nine-color timeline palette.
    const firstColor = result.current.first('a');
    const secondColor = result.current.second('j');
    expect(COLOR_PALETTES.timeline.dark).toContain(firstColor);
    expect(COLOR_PALETTES.timeline.dark).toContain(secondColor);
    expect(secondColor).not.toBe(firstColor);
    expect(result.current.second('a')).toBe(firstColor);
  });

  it('assigns collision-aware colors lazily from an empty registry', () => {
    const palette = ['#111111', '#222222'];
    const registry: ColorRegistry = new Map([
      [COLOR_REGISTRY_KEYS.DATA_FLOW_DIMENSIONS, { colorMap: new Map(), palette }],
    ]);
    const { result } = renderHook(
      () => ({
        first: useColorResolver(COLOR_REGISTRY_KEYS.DATA_FLOW_DIMENSIONS, []),
        second: useColorResolver(COLOR_REGISTRY_KEYS.DATA_FLOW_DIMENSIONS, []),
      }),
      { wrapper: createWrapper(registry) }
    );

    const firstColor = result.current.first('a');
    const secondColor = result.current.second('c');

    expect(firstColor).not.toBe(secondColor);
    expect(result.current.second('a')).toBe(firstColor);
    expect(result.current.first('c')).toBe(secondColor);
  });
});
