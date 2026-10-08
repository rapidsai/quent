// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo, type ReactNode } from 'react';
import { COLOR_REGISTRY_KEYS, ColorRegistryProvider } from '@quent/hooks';
import {
  createColorRegistry,
  createColorRegistryEntry,
  type ColorRegistry,
  type EntityRef,
  type PaletteTheme,
  type QueryBundle,
} from '@quent/utils';

export function QueryColorRegistry({
  queryBundle,
  paletteTheme,
  children,
}: {
  queryBundle: QueryBundle<EntityRef>;
  paletteTheme: PaletteTheme;
  children: ReactNode;
}) {
  const registry = useMemo<ColorRegistry>(() => {
    const resourceTypes = Object.values(queryBundle.entities.resource_types);
    const fsmTypes = Object.values(queryBundle.entities.fsm_types);
    const fsmStates = fsmTypes.flatMap(type => type.states.map(state => state.name));

    return createColorRegistry(
      [
        createColorRegistryEntry(
          COLOR_REGISTRY_KEYS.OPERATOR_TYPES,
          queryBundle.unique_operator_names
        ),
        createColorRegistryEntry(
          COLOR_REGISTRY_KEYS.RESOURCE_TYPES,
          resourceTypes,
          type => type.name
        ),
        createColorRegistryEntry(COLOR_REGISTRY_KEYS.FSM_TYPES, fsmTypes, type => type.name),
        createColorRegistryEntry(
          COLOR_REGISTRY_KEYS.CAPACITIES,
          resourceTypes.flatMap(type => type.capacities.map(capacity => capacity.name))
        ),
        createColorRegistryEntry(COLOR_REGISTRY_KEYS.FSM_STATES, fsmStates),
      ],
      paletteTheme
    );
  }, [paletteTheme, queryBundle]);
  return <ColorRegistryProvider registry={registry}>{children}</ColorRegistryProvider>;
}
