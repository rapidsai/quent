// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  buildDeterministicColorMap,
  COLOR_PALETTES,
  createDeterministicColorResolver,
  extendDeterministicColorMap,
  type ColorPalette,
  type DeterministicColorKey,
  type DeterministicColorResolver,
  type PaletteTheme,
} from './colors';

export const COLOR_REGISTRY_KEYS = {
  OPERATOR_TYPES: 'operator-types',
  RESOURCE_TYPES: 'resource-types',
  FSM_TYPES: 'fsm-types',
  CAPACITIES: 'capacities',
  FSM_STATES: 'fsm-states',
  DATA_FLOW_DIMENSIONS: 'data-flow-dimensions',
} as const;

export type ColorRegistryKey = (typeof COLOR_REGISTRY_KEYS)[keyof typeof COLOR_REGISTRY_KEYS];
export type ColorMap = ReadonlyMap<string, string>;
export type ColorResolver = (value: string) => string;
export type ColorRegistryValue = {
  colorMap: ColorMap;
  palette: ColorPalette;
};
export type ColorRegistry = ReadonlyMap<ColorRegistryKey, ColorRegistryValue>;
export type ColorRegistryEntry = {
  registryKey: ColorRegistryKey;
  createColorMap: (palette: ColorPalette) => ColorMap;
};
export type ColorRegistryPalettes = Record<ColorRegistryKey, ColorPalette>;
type HydratedColorRegistryEntry = readonly [ColorRegistryKey, ColorRegistryValue];

const DEFAULT_REGISTRY_VALUE: ColorRegistryValue = {
  colorMap: new Map(),
  palette: COLOR_PALETTES.deterministic,
};

export function getColorRegistryPalettes(theme: PaletteTheme): ColorRegistryPalettes {
  const timelinePalette = COLOR_PALETTES.timeline[theme];
  return {
    [COLOR_REGISTRY_KEYS.OPERATOR_TYPES]: COLOR_PALETTES.deterministic,
    [COLOR_REGISTRY_KEYS.RESOURCE_TYPES]: COLOR_PALETTES.deterministic,
    [COLOR_REGISTRY_KEYS.FSM_TYPES]: COLOR_PALETTES.deterministic,
    [COLOR_REGISTRY_KEYS.CAPACITIES]: timelinePalette,
    [COLOR_REGISTRY_KEYS.FSM_STATES]: timelinePalette,
    [COLOR_REGISTRY_KEYS.DATA_FLOW_DIMENSIONS]: timelinePalette,
  };
}

export function createColorRegistryEntry(
  registryKey: ColorRegistryKey,
  values: Iterable<DeterministicColorKey>
): ColorRegistryEntry;
export function createColorRegistryEntry<T>(
  registryKey: ColorRegistryKey,
  values: Iterable<T>,
  keyOf: (value: T) => DeterministicColorKey
): ColorRegistryEntry;
export function createColorRegistryEntry<T>(
  registryKey: ColorRegistryKey,
  values: Iterable<T>,
  keyOf?: (value: T) => DeterministicColorKey
): ColorRegistryEntry {
  return {
    registryKey,
    createColorMap: palette =>
      keyOf
        ? buildDeterministicColorMap(values, keyOf, palette)
        : buildDeterministicColorMap(values as Iterable<DeterministicColorKey>, palette),
  };
}

class LazyColorRegistry extends Map<ColorRegistryKey, ColorRegistryValue> {
  constructor(
    entries: Iterable<HydratedColorRegistryEntry>,
    private readonly palettes: ColorRegistryPalettes
  ) {
    super(entries);
  }

  override get(key: ColorRegistryKey): ColorRegistryValue {
    let value = super.get(key);
    if (!value) {
      value = { colorMap: new Map(), palette: this.palettes[key] };
      this.set(key, value);
    }
    return value;
  }
}

/** Creates registry entries on first use, with palettes selected by theme. */
export function createColorRegistry(
  entries: Iterable<ColorRegistryEntry> = [],
  theme: PaletteTheme = 'light'
): ColorRegistry {
  const palettes = getColorRegistryPalettes(theme);
  const hydratedEntries = [...entries].map(({ registryKey, createColorMap }) => {
    const palette = palettes[registryKey];
    return [registryKey, { colorMap: createColorMap(palette), palette }] as const;
  });
  return new LazyColorRegistry(hydratedEntries, palettes);
}

export function createRegistryColorResolver(
  registry: ColorRegistry,
  registryKey: ColorRegistryKey,
  additionalValues: Iterable<DeterministicColorKey> = []
): DeterministicColorResolver {
  const { colorMap, palette } = registry.get(registryKey) ?? DEFAULT_REGISTRY_VALUE;
  return createDeterministicColorResolver(
    extendDeterministicColorMap(colorMap, additionalValues, palette),
    palette
  );
}
