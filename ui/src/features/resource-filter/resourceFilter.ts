// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { TreeTableItem } from '@quent/components';
import { EntityTypeKey, type QueryEntities } from '@quent/utils';

export const MAX_RESOURCE_FILTER_QUERY_LENGTH = 512;

export interface ResourceFilter {
  fsmTypes: string[];
  resourceTypes: string[];
  search: string;
  showOthers: boolean;
}

export const EMPTY_RESOURCE_FILTER: ResourceFilter = {
  fsmTypes: [],
  resourceTypes: [],
  search: '',
  showOthers: false,
};

export interface ResourceFilterResult {
  directMatchIds: Set<string>;
  filteredItems: TreeTableItem[];
  isActive: boolean;
  matchCount: number;
}

type ResourceTypeEntities = Pick<QueryEntities, 'resource_types'>;

export function isResourceFilterActive(filter: ResourceFilter): boolean {
  return (
    filter.search.trim().length > 0 || filter.resourceTypes.length > 0 || filter.fsmTypes.length > 0
  );
}

function itemMatches(
  item: TreeTableItem,
  filter: ResourceFilter,
  entities: ResourceTypeEntities
): boolean {
  const entity = item.entity;
  const name = 'instance_name' in entity ? (entity.instance_name ?? '') : '';
  const typeName = 'type_name' in entity ? (entity.type_name ?? '') : '';
  const isResource = item.type === EntityTypeKey.Resource;
  const fsmTypes = isResource ? (entities.resource_types[typeName]?.used_by ?? []) : [];
  const searchTermGroups = filter.search
    .split(',')
    .map(group => group.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean))
    .filter(group => group.length > 0);
  const searchableLabel = `${item.id} ${name} ${typeName}`.toLocaleLowerCase();

  if (
    searchTermGroups.length > 0 &&
    !searchTermGroups.some(group => group.every(term => searchableLabel.includes(term)))
  ) {
    return false;
  }
  if (
    filter.resourceTypes.length > 0 &&
    (!isResource ||
      !filter.resourceTypes.some(
        resourceType => typeName.toLocaleLowerCase() === resourceType.toLocaleLowerCase()
      ))
  ) {
    return false;
  }
  if (
    filter.fsmTypes.length > 0 &&
    (!isResource ||
      !fsmTypes.some(fsmType =>
        filter.fsmTypes.some(
          selectedFsmType => fsmType.toLocaleLowerCase() === selectedFsmType.toLocaleLowerCase()
        )
      ))
  ) {
    return false;
  }
  return true;
}

export function filterResourceTree(
  root: TreeTableItem,
  entities: ResourceTypeEntities,
  filter: ResourceFilter
): ResourceFilterResult {
  const directMatchIds = new Set<string>();
  let matchCount = 0;
  const isActive = isResourceFilterActive(filter);

  if (!isActive) {
    return {
      directMatchIds,
      filteredItems: [root],
      isActive,
      matchCount,
    };
  }

  const visit = (item: TreeTableItem): TreeTableItem[] => {
    const isDirectMatch = itemMatches(item, filter, entities);
    const matchedChildren = item.children?.flatMap(visit) ?? [];
    if (isDirectMatch) {
      directMatchIds.add(item.id);
      matchCount += 1;
      return [{ ...item, children: matchedChildren }];
    }
    return matchedChildren;
  };

  return {
    directMatchIds,
    filteredItems: visit(root),
    isActive,
    matchCount,
  };
}
