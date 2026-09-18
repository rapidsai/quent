// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { atom, useAtomValue, useSetAtom } from 'jotai';
import { useHydrateAtoms } from 'jotai/utils';
import { useEffect, useMemo } from 'react';
import {
  COLOR_PALETTES,
  createColorRegistry,
  extendDeterministicColorMap,
  getDeterministicColorFromPalette,
  normalizeDeterministicColorKey,
  type ColorPalette,
  type ColorRegistry,
  type ColorRegistryKey,
  type DeterministicColorKey,
  type DeterministicColorResolver,
} from '@quent/utils';

export { COLOR_REGISTRY_KEYS } from '@quent/utils';
export type { ColorRegistry, ColorRegistryKey } from '@quent/utils';

const colorRegistryAtom = atom<ColorRegistry>(createColorRegistry());
const colorResolverCacheAtom = atom(
  () => new WeakMap<ColorRegistry, Map<ColorRegistryKey, IncrementalColorResolver>>()
);
const EMPTY_ADDITIONAL_VALUES: readonly DeterministicColorKey[] = [];

type IncrementalColorResolver = {
  addValues: (values: Iterable<DeterministicColorKey>) => void;
  resolveColor: DeterministicColorResolver;
};

function createIncrementalColorResolver(
  initialColorMap: ReadonlyMap<string, string>,
  palette: ColorPalette,
  assignMissingValues: boolean
): IncrementalColorResolver {
  let colorMap = new Map(initialColorMap);
  const addValues = (values: Iterable<DeterministicColorKey>) => {
    colorMap = extendDeterministicColorMap(colorMap, values, palette);
  };
  const resolveColor = ((
    value: DeterministicColorKey,
    keyOf?: (value: unknown) => DeterministicColorKey
  ) => {
    const key = normalizeDeterministicColorKey(keyOf ? keyOf(value) : value);
    const color = colorMap.get(key);
    if (color !== undefined) {
      return color;
    }
    if (!assignMissingValues) {
      return getDeterministicColorFromPalette(key, palette);
    }
    addValues([key]);
    return colorMap.get(key)!;
  }) as DeterministicColorResolver;
  return { addValues, resolveColor };
}

function getIncrementalColorResolver(
  cache: WeakMap<ColorRegistry, Map<ColorRegistryKey, IncrementalColorResolver>>,
  registry: ColorRegistry,
  registryKey: ColorRegistryKey
): IncrementalColorResolver {
  let registryCache = cache.get(registry);
  if (!registryCache) {
    registryCache = new Map();
    cache.set(registry, registryCache);
  }

  let resolver = registryCache.get(registryKey);
  if (!resolver) {
    const registryValue = registry.get(registryKey);
    resolver = createIncrementalColorResolver(
      registryValue?.colorMap ?? new Map(),
      registryValue?.palette ?? COLOR_PALETTES.deterministic,
      registryValue !== undefined
    );
    registryCache.set(registryKey, resolver);
  }
  return resolver;
}

export function useColorResolver(
  registryKey: ColorRegistryKey,
  additionalValues: Iterable<DeterministicColorKey> = EMPTY_ADDITIONAL_VALUES
): DeterministicColorResolver {
  const registry = useAtomValue(colorRegistryAtom);
  const cache = useAtomValue(colorResolverCacheAtom);
  return useMemo(() => {
    const resolver = getIncrementalColorResolver(cache, registry, registryKey);
    resolver.addValues(additionalValues);
    return resolver.resolveColor;
  }, [additionalValues, cache, registry, registryKey]);
}

/** Hydrates complete color maps before descendants read their resolvers. */
export function useHydrateColorRegistry(registry: ColorRegistry): void {
  useHydrateAtoms([[colorRegistryAtom, registry]]);
  const setColorRegistry = useSetAtom(colorRegistryAtom);
  useEffect(() => {
    setColorRegistry(registry);
  }, [registry, setColorRegistry]);
}
