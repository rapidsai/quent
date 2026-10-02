// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  formatStatWithQuantity,
  isStatStruct,
  type QuantitySpec,
  type Statistic,
  type StatValue,
} from '@quent/utils';

export interface StatisticSectionAction {
  label: string;
  enter: () => void;
  leave: () => void;
  activate: () => void;
}
type SectionActions = ReadonlyMap<string, StatisticSectionAction>;
type Quantities = { [key: string]: QuantitySpec | undefined };

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
    <span className="break-words whitespace-pre-wrap">
      {value == null
        ? '—'
        : typeof value === 'number' || typeof value === 'bigint'
          ? formatStatWithQuantity(value, name, quantity ? quantitySpecs?.[quantity] : undefined)
          : String(value)}
    </span>
  );
}

/** Render producer-owned section names and field order without semantic classification. */
export function StatisticFields({
  statistics,
  quantitySpecs,
  depth = 0,
  path = [],
  sectionActions,
}: {
  statistics: readonly Statistic[];
  quantitySpecs?: Quantities;
  depth?: number;
  path?: readonly string[];
  sectionActions?: SectionActions;
}) {
  if (!statistics.length) {
    return <p className="text-xs text-muted-foreground">No fields</p>;
  }
  return (
    <div className="space-y-0.5 text-xs leading-snug">
      {statistics.map(({ key, value, quantity }, index) => {
        const nextPath = [...path, key];
        const action = sectionActions?.get(JSON.stringify(nextPath));
        return isStatStruct(value) ? (
          <section
            key={index}
            aria-label={action?.label ?? key}
            role={action ? 'button' : undefined}
            tabIndex={action ? 0 : undefined}
            onMouseEnter={action?.enter}
            onMouseLeave={action?.leave}
            onFocus={action?.enter}
            onBlur={action?.leave}
            onClick={action?.activate}
            onKeyDown={
              action
                ? event => {
                    if (event.key === 'Enter' || event.key === ' ') {
                      event.preventDefault();
                      action.activate();
                    }
                  }
                : undefined
            }
            className={`mt-2 border-t pt-1 first:mt-0.5 ${action ? 'cursor-pointer rounded-sm hover:bg-muted/50 focus-visible:outline focus-visible:outline-2 focus-visible:outline-primary' : ''}`}
          >
            <div
              role="heading"
              aria-level={Math.min(6, depth + 4)}
              className="mb-1 break-words font-semibold"
            >
              {key}
            </div>
            <div className="border-l pl-3">
              <StatisticFields
                statistics={value.fields}
                quantitySpecs={quantitySpecs}
                depth={depth + 1}
                path={nextPath}
                sectionActions={sectionActions}
              />
            </div>
          </section>
        ) : (
          <dl
            key={index}
            className="grid grid-cols-[minmax(0,1fr)_minmax(0,1.5fr)] items-start gap-x-4 py-0"
          >
            <dt className="break-words">{key.replace(/_/g, ' ')}:</dt>
            <dd className="min-w-0 text-muted-foreground">
              <Value
                name={key}
                value={value}
                quantity={quantity}
                quantitySpecs={quantitySpecs}
                depth={depth + 1}
              />
            </dd>
          </dl>
        );
      })}
    </div>
  );
}
