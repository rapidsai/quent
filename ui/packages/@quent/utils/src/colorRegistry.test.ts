// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { COLOR_PALETTES, buildDeterministicColorMap, getDeterministicColor } from './colors';
import {
  COLOR_REGISTRY_KEYS,
  createColorRegistry,
  createColorRegistryEntry,
  createRegistryColorResolver,
  getColorRegistryPalettes,
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
      COLOR_REGISTRY_KEYS.DATA_FLOW_STATES,
      COLOR_REGISTRY_KEYS.DATA_FLOW_DIMENSIONS,
    ]) {
      expect(light[key]).toBe(COLOR_PALETTES.timeline.light);
      expect(dark[key]).toBe(COLOR_PALETTES.timeline.dark);
    }
  });

  it('resolves colors within independent registry keys', () => {
    const registry = createColorRegistry([
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

  it('uses deterministic fallback colors for missing maps and values', () => {
    const registry = createColorRegistry([
      createColorRegistryEntry(
        COLOR_REGISTRY_KEYS.FSM_STATES,
        ['running'],
        COLOR_PALETTES.deterministic
      ),
    ]);
    const resolveColor = createRegistryColorResolver(registry, COLOR_REGISTRY_KEYS.FSM_STATES);

    expect(resolveColor('unknown')).toBe(getDeterministicColor('unknown'));
    expect(createRegistryColorResolver(registry, COLOR_REGISTRY_KEYS.CAPACITIES)('unknown')).toBe(
      getDeterministicColor('unknown')
    );
  });

  it('adds runtime values without changing hydrated assignments', () => {
    const stateColors = buildDeterministicColorMap(['declared-a', 'declared-b']);
    const registry = createColorRegistry([
      [
        COLOR_REGISTRY_KEYS.DATA_FLOW_STATES,
        { colorMap: stateColors, palette: COLOR_PALETTES.deterministic },
      ],
    ]);
    const resolveColor = createRegistryColorResolver(
      registry,
      COLOR_REGISTRY_KEYS.DATA_FLOW_STATES,
      ['synthetic']
    );

    expect(resolveColor('declared-a')).toBe(stateColors.get('declared-a'));
    expect([...stateColors.values()]).not.toContain(resolveColor('synthetic'));
  });

  it('uses each registry key palette for maps, extensions, and fallbacks', () => {
    const operatorPalette = ['#111111', '#222222'];
    const timelinePalette = ['#aaaaaa', '#bbbbbb'];
    const registry = createColorRegistry([
      createColorRegistryEntry(COLOR_REGISTRY_KEYS.OPERATOR_TYPES, ['scan'], operatorPalette),
      createColorRegistryEntry(COLOR_REGISTRY_KEYS.FSM_STATES, ['running'], timelinePalette),
    ]);
    const resolveOperator = createRegistryColorResolver(
      registry,
      COLOR_REGISTRY_KEYS.OPERATOR_TYPES,
      ['join']
    );
    const resolveState = createRegistryColorResolver(registry, COLOR_REGISTRY_KEYS.FSM_STATES, [
      'waiting',
    ]);

    expect(operatorPalette).toContain(resolveOperator('scan'));
    expect(operatorPalette).toContain(resolveOperator('join'));
    expect(operatorPalette).toContain(resolveOperator('unknown'));
    expect(timelinePalette).toContain(resolveState('running'));
    expect(timelinePalette).toContain(resolveState('waiting'));
    expect(timelinePalette).toContain(resolveState('unknown'));
  });
});
