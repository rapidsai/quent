// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useAtom } from 'jotai';
import { resourceFilterAtom } from '@/atoms/resourceTree';
import { ResourceFilterSearch } from '@/features/resource-filter/ResourceFilterSearch';

interface TimelineTreeFilterControlsProps {
  fsmTypes: string[];
  matchCount: number;
  matchIds: readonly string[];
  onPinMany: (ids: string[]) => void;
  pinnedIds: ReadonlySet<string>;
  resourceTypes: string[];
}

export function TimelineTreeFilterControls({
  fsmTypes,
  matchCount,
  matchIds,
  onPinMany,
  pinnedIds,
  resourceTypes,
}: TimelineTreeFilterControlsProps) {
  const [filter, setFilter] = useAtom(resourceFilterAtom);

  return (
    <ResourceFilterSearch
      fsmTypes={fsmTypes}
      matchCount={matchCount}
      matchIds={matchIds}
      onFsmTypesChange={fsmTypes => setFilter(current => ({ ...current, fsmTypes }))}
      onPinMany={onPinMany}
      onResourceTypesChange={resourceTypes => setFilter(current => ({ ...current, resourceTypes }))}
      onSearchChange={search => setFilter(current => ({ ...current, search }))}
      onShowOthersChange={showOthers => setFilter(current => ({ ...current, showOthers }))}
      pinnedIds={pinnedIds}
      resourceTypes={resourceTypes}
      search={filter.search}
      selectedFsmTypes={filter.fsmTypes}
      selectedResourceTypes={filter.resourceTypes}
      showOthers={filter.showOthers}
    />
  );
}
