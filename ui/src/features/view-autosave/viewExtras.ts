// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { useStore } from 'jotai';
import { LONG_ENTITY_DENSITY_DEFAULT, type LongEntityDensity } from '@quent/hooks';
import {
  pinnedCollapsedAtom,
  pinnedPanelSizeAtom,
  selectedNvtxCategoriesAtom,
  selectedNvtxDomainAtom,
} from '@/atoms/resourceTree';
import type { ViewExtras } from './viewAutosave.schema';

type Store = ReturnType<typeof useStore>;

function hasKeys(value: object): boolean {
  return Object.keys(value).length > 0;
}

/** Reads the view state that shared links do not carry. Defaults are omitted. */
export function captureViewExtras(store: Store, density: LongEntityDensity): ViewExtras {
  const extras: ViewExtras = {};
  if (store.get(pinnedCollapsedAtom)) {
    extras.pinnedCollapsed = true;
  }
  const pinnedPanelSize = store.get(pinnedPanelSizeAtom);
  if (pinnedPanelSize !== undefined && pinnedPanelSize > 0) {
    extras.pinnedPanelSize = Math.round(pinnedPanelSize);
  }
  const domainId = store.get(selectedNvtxDomainAtom);
  const categories = [...store.get(selectedNvtxCategoriesAtom)]
    .map(([categoryDomainId, categoryId]) => ({ domainId: categoryDomainId, categoryId }))
    .sort((a, b) => a.domainId.localeCompare(b.domainId));
  if (domainId || categories.length > 0) {
    extras.nvtx = {
      ...(domainId ? { domainId } : {}),
      ...(categories.length > 0 ? { categories } : {}),
    };
  }
  if (density !== LONG_ENTITY_DENSITY_DEFAULT) {
    extras.longEntityDensity = density;
  }
  return hasKeys(extras) ? extras : {};
}

/** Writes saved extras back into the store. Anything absent returns to its default. */
export function hydrateViewExtras(store: Store, extras: ViewExtras): void {
  store.set(pinnedCollapsedAtom, extras.pinnedCollapsed ?? false);
  store.set(pinnedPanelSizeAtom, extras.pinnedPanelSize);
  store.set(selectedNvtxDomainAtom, extras.nvtx?.domainId ?? null);
  store.set(
    selectedNvtxCategoriesAtom,
    new Map(
      extras.nvtx?.categories?.map(({ domainId, categoryId }) => [domainId, categoryId]) ?? []
    )
  );
}
