// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createContext, createElement, useContext, useMemo, useState, type ReactNode } from 'react';
import {
  COLOR_PALETTES,
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

const EMPTY_ADDITIONAL_VALUES: readonly DeterministicColorKey[] = [];
const EMPTY_COLOR_REGISTRY: ColorRegistry = new Map();

type IncrementalColorResolver = {
  addValues: (values: Iterable<DeterministicColorKey>) => void;
  resolveColor: DeterministicColorResolver;
};

type ColorRegistryContextValue = {
  registry: ColorRegistry;
  resolverCache: Map<ColorRegistryKey, IncrementalColorResolver>;
};

const ColorRegistryContext = createContext<ColorRegistryContextValue>({
  registry: EMPTY_COLOR_REGISTRY,
  resolverCache: new Map(),
});

export function ColorRegistryProvider({
  registry = EMPTY_COLOR_REGISTRY,
  children,
}: {
  registry?: ColorRegistry;
  children: ReactNode;
}) {
  const [resolverCaches] = useState(
    () => new WeakMap<ColorRegistry, Map<ColorRegistryKey, IncrementalColorResolver>>()
  );
  let resolverCache = resolverCaches.get(registry);
  if (!resolverCache) {
    resolverCache = new Map();
    resolverCaches.set(registry, resolverCache);
  }
  const value = useMemo<ColorRegistryContextValue>(
    () => ({ registry, resolverCache }),
    [registry, resolverCache]
  );
  return createElement(ColorRegistryContext.Provider, { value }, children);
}

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
  cache: Map<ColorRegistryKey, IncrementalColorResolver>,
  registry: ColorRegistry,
  registryKey: ColorRegistryKey
): IncrementalColorResolver {
  let resolver = cache.get(registryKey);
  if (!resolver) {
    const registryValue = registry.get(registryKey);
    resolver = createIncrementalColorResolver(
      registryValue?.colorMap ?? new Map(),
      registryValue?.palette ?? COLOR_PALETTES.deterministic,
      registryValue !== undefined
    );
    cache.set(registryKey, resolver);
  }
  return resolver;
}

export function useColorResolver(
  registryKey: ColorRegistryKey,
  additionalValues: Iterable<DeterministicColorKey> = EMPTY_ADDITIONAL_VALUES
): DeterministicColorResolver {
  const { registry, resolverCache } = useContext(ColorRegistryContext);
  return useMemo(() => {
    const resolver = getIncrementalColorResolver(resolverCache, registry, registryKey);
    resolver.addValues(additionalValues);
    return resolver.resolveColor;
  }, [additionalValues, registry, registryKey, resolverCache]);
}
