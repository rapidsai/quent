// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { formatQuantity, inferFieldFormatter } from '@quent/utils';
import type { EntityRef, FsmUsage, QueryBundle } from '@quent/utils';
import { DataText } from '@quent/components';

interface ResourceUsageListProps {
  usages: FsmUsage[];
  resourceLabel: (id: string) => string;
  queryBundle: QueryBundle<EntityRef>;
}

export function ResourceUsageList({ usages, resourceLabel, queryBundle }: ResourceUsageListProps) {
  if (usages.length === 0) return null;

  return (
    <ul className="mt-1.5 space-y-1.5">
      {usages.map((usage, usageIndex) => {
        const resourceTypeName = queryBundle.entities.resources[usage.resource]?.type_name;
        const resourceType = resourceTypeName
          ? queryBundle.entities.resource_types[resourceTypeName]
          : undefined;

        return (
          <li key={`${usage.resource}-${usageIndex}`} className="px-1 py-1">
            <h4 className="truncate border-b pb-1 font-mono text-xs font-medium">
              <DataText>{resourceLabel(usage.resource)}</DataText>
            </h4>
            {usage.capacities.length > 0 && (
              <dl className="mt-1 space-y-0.5 text-xs">
                {usage.capacities.map(([name, capacity], capacityIndex) => {
                  const capacityDecl = resourceType?.capacities.find(item => item.name === name);
                  const quantitySpec = capacityDecl
                    ? queryBundle.quantity_specs[capacityDecl.quantity]
                    : undefined;

                  return (
                    <div
                      key={`${name}-${capacityIndex}`}
                      className="flex items-baseline justify-between gap-3"
                    >
                      <dt className="text-muted-foreground">
                        <DataText>{name}</DataText>
                      </dt>
                      <dd className="tabular-nums">
                        <DataText>
                          {capacity == null
                            ? '—'
                            : capacityDecl && quantitySpec
                              ? formatQuantity(capacity, quantitySpec, capacityDecl.kind)
                              : inferFieldFormatter(name)(capacity)}
                        </DataText>
                      </dd>
                    </div>
                  );
                })}
              </dl>
            )}
          </li>
        );
      })}
    </ul>
  );
}
