// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  formatStatWithQuantity,
  isStatStruct,
  type QuantitySpec,
  type Statistic,
  type StatValue,
} from '@quent/utils';
import type { ReactNode } from 'react';
import { DataText } from '../ui/data-text';

type Quantities = { [key: string]: QuantitySpec | undefined };

export function StatisticField({ name, children }: { name: string; children: ReactNode }) {
  return (
    <dl className="grid grid-cols-[minmax(0,1fr)_minmax(0,1.5fr)] items-start gap-x-4 py-0 text-xs leading-snug">
      <DataText as="dt" className="break-words">
        {name.replace(/_/g, ' ')}:
      </DataText>
      <dd className="min-w-0 text-muted-foreground">{children}</dd>
    </dl>
  );
}

function Value({
  name,
  value,
  quantity,
  quantitySpecs,
  depth,
}: {
  name: string;
  value: StatValue;
  quantity?: string;
  quantitySpecs?: Quantities;
  depth: number;
}) {
  if (isStatStruct(value)) {
    return (
      <StatisticFields statistics={value.fields} quantitySpecs={quantitySpecs} depth={depth} />
    );
  }
  if (Array.isArray(value)) {
    return value.length ? (
      <ol className="space-y-1" aria-label={`${name} values`}>
        {value.map((item, index) => (
          <li key={index} className="break-words whitespace-pre-wrap">
            <Value
              name={name}
              value={item}
              quantity={quantity}
              quantitySpecs={quantitySpecs}
              depth={depth}
            />
          </li>
        ))}
      </ol>
    ) : (
      <span className="text-muted-foreground">Empty list</span>
    );
  }
  return (
    <DataText className="break-words whitespace-pre-wrap">
      {value == null
        ? '—'
        : typeof value === 'number' || typeof value === 'bigint'
          ? formatStatWithQuantity(value, name, quantity ? quantitySpecs?.[quantity] : undefined)
          : String(value)}
    </DataText>
  );
}

/** Render producer-owned section names and field order without semantic classification. */
export function StatisticFields({
  statistics,
  quantitySpecs,
  depth = 0,
}: {
  statistics: readonly Statistic[];
  quantitySpecs?: Quantities;
  depth?: number;
}) {
  if (!statistics.length) {
    return <p className="text-xs text-muted-foreground">No fields</p>;
  }
  return (
    <div className="space-y-0.5 text-xs leading-snug">
      {statistics.map(({ key, value, quantity }, index) =>
        isStatStruct(value) ? (
          <section key={index} className="mt-2 border-t pt-1 first:mt-0.5">
            <DataText
              as="div"
              role="heading"
              aria-level={Math.min(6, depth + 4)}
              className="mb-1 break-words font-semibold"
            >
              {key}
            </DataText>
            <div className="border-l pl-3">
              <StatisticFields
                statistics={value.fields}
                quantitySpecs={quantitySpecs}
                depth={depth + 1}
              />
            </div>
          </section>
        ) : (
          <StatisticField key={index} name={key}>
            <Value
              name={key}
              value={value}
              quantity={quantity}
              quantitySpecs={quantitySpecs}
              depth={depth + 1}
            />
          </StatisticField>
        )
      )}
    </div>
  );
}
