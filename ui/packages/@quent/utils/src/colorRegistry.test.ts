// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { COLOR_PALETTES, getDeterministicColorFromPalette } from './colors';
import {
  COLOR_REGISTRY_KEYS,
  createColorRegistry,
  createColorRegistryEntry,
  createRegistryColorResolver,
  getColorRegistryPalettes,
  type ColorRegistry,
} from './colorRegistry';

describe('color registry core', () => {
  it('matches each registry key to its pre-registry palette', () => {
    const light = getColorRegistryPalettes('light');
    const dark = getColorRegistryPalettes('dark');

    for (const key of [
      COLOR_REGISTRY_KEYS.OPERATOR_TYPES,
      COLOR_REGISTRY_KEYS.RESOURCE_TYPES,
      COLOR_REGISTRY_KEYS.FSM_TYPES,
    ]) {
      expect(light[key]).toBe(COLOR_PALETTES.deterministic);
      expect(dark[key]).toBe(COLOR_PALETTES.deterministic);
    }
    for (const key of [
      COLOR_REGISTRY_KEYS.CAPACITIES,
      COLOR_REGISTRY_KEYS.FSM_STATES,
      COLOR_REGISTRY_KEYS.DATA_FLOW_DIMENSIONS,
    ]) {
      expect(light[key]).toBe(COLOR_PALETTES.timeline.light);
      expect(dark[key]).toBe(COLOR_PALETTES.timeline.dark);
    }
  });

  it('initializes omitted keys only when read, using the selected theme', () => {
    const registry = createColorRegistry([], 'dark');
    expect(registry.size).toBe(0);
    const value = registry.get(COLOR_REGISTRY_KEYS.DATA_FLOW_DIMENSIONS);
    expect(value?.palette).toBe(COLOR_PALETTES.timeline.dark);
    expect(registry.size).toBe(1);
    expect(registry.get(COLOR_REGISTRY_KEYS.DATA_FLOW_DIMENSIONS)).toBe(value);
  });

  it('resolves colors within independent registry keys', () => {
    const registry: ColorRegistry = new Map([
      [
        COLOR_REGISTRY_KEYS.OPERATOR_TYPES,
        {
          colorMap: new Map([['shared', '#111111']]),
          palette: COLOR_PALETTES.deterministic,
        },
      ],
      [
        COLOR_REGISTRY_KEYS.RESOURCE_TYPES,
        {
          colorMap: new Map([['shared', '#222222']]),
          palette: COLOR_PALETTES.deterministic,
        },
      ],
    ]);

    expect(
      createRegistryColorResolver(registry, COLOR_REGISTRY_KEYS.OPERATOR_TYPES)('shared')
    ).toBe('#111111');
    expect(
      createRegistryColorResolver(registry, COLOR_REGISTRY_KEYS.RESOURCE_TYPES)('shared')
    ).toBe('#222222');
  });

  it('uses the registry palette for missing values', () => {
    const registry = createColorRegistry([
      createColorRegistryEntry(COLOR_REGISTRY_KEYS.FSM_STATES, ['running']),
    ]);
    const resolveColor = createRegistryColorResolver(registry, COLOR_REGISTRY_KEYS.FSM_STATES);

    expect(resolveColor('unknown')).toBe(
      getDeterministicColorFromPalette('unknown', COLOR_PALETTES.timeline.light)
    );
    expect(createRegistryColorResolver(registry, COLOR_REGISTRY_KEYS.CAPACITIES)('unknown')).toBe(
      getDeterministicColorFromPalette('unknown', COLOR_PALETTES.timeline.light)
    );
  });

  it('adds runtime values without changing hydrated assignments', () => {
    const registry = createColorRegistry([
      createColorRegistryEntry(COLOR_REGISTRY_KEYS.DATA_FLOW_DIMENSIONS, [
        'declared-a',
        'declared-b',
      ]),
    ]);
    const declaredColors = registry.get(COLOR_REGISTRY_KEYS.DATA_FLOW_DIMENSIONS)!.colorMap;
    const resolveColor = createRegistryColorResolver(
      registry,
      COLOR_REGISTRY_KEYS.DATA_FLOW_DIMENSIONS,
      ['synthetic']
    );

    expect(resolveColor('declared-a')).toBe(declaredColors.get('declared-a'));
    expect([...declaredColors.values()]).not.toContain(resolveColor('synthetic'));
  });

  it('uses each registry key palette for maps, extensions, and fallbacks', () => {
    const registry = createColorRegistry(
      [
        createColorRegistryEntry(COLOR_REGISTRY_KEYS.OPERATOR_TYPES, ['scan']),
        createColorRegistryEntry(COLOR_REGISTRY_KEYS.FSM_STATES, ['running']),
      ],
      'dark'
    );
    const resolveOperator = createRegistryColorResolver(
      registry,
      COLOR_REGISTRY_KEYS.OPERATOR_TYPES,
      ['join']
    );
    const resolveState = createRegistryColorResolver(registry, COLOR_REGISTRY_KEYS.FSM_STATES, [
      'waiting',
    ]);

    expect(COLOR_PALETTES.deterministic).toContain(resolveOperator('scan'));
    expect(COLOR_PALETTES.deterministic).toContain(resolveOperator('join'));
    expect(COLOR_PALETTES.deterministic).toContain(resolveOperator('unknown'));
    expect(COLOR_PALETTES.timeline.dark).toContain(resolveState('running'));
    expect(COLOR_PALETTES.timeline.dark).toContain(resolveState('waiting'));
    expect(COLOR_PALETTES.timeline.dark).toContain(resolveState('unknown'));
  });
});
