// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { cn } from '@quent/utils';

const MAX_CATEGORICAL_ENTRIES = 8;

export interface CategoricalLegendProps {
  label: string;
  categoryMap: ReadonlyMap<string, string>;
  dimmedLabels?: ReadonlySet<string>;
  entrySuffixes?: ReadonlyMap<string, string>;
}

export function CategoricalLegend({
  label,
  categoryMap,
  dimmedLabels,
  entrySuffixes,
}: CategoricalLegendProps) {
  const entries = [...categoryMap.entries()].slice(0, MAX_CATEGORICAL_ENTRIES);
  const truncated = categoryMap.size > MAX_CATEGORICAL_ENTRIES;
  return (
    <div className="flex min-w-0 flex-col gap-1">
      <span
        className="truncate text-[10px] font-semibold uppercase tracking-wide text-muted-foreground"
        title={label}
      >
        {label}
      </span>
      <div className="flex flex-col gap-0.5">
        {entries.map(([entryLabel, color]) => {
          const dimmed = dimmedLabels?.has(entryLabel) ?? false;
          const suffix = entrySuffixes?.get(entryLabel);
          return (
            <div
              key={entryLabel}
              data-dimmed={dimmed || undefined}
              className={cn('flex items-center gap-1.5', dimmed && 'opacity-40')}
            >
              <span
                className="inline-block h-2.5 w-2.5 shrink-0 rounded-sm"
                style={{ backgroundColor: color }}
              />
              <span
                className={cn(
                  'max-w-[120px] truncate text-[10px] text-muted-foreground',
                  dimmed && 'line-through'
                )}
                title={entryLabel}
              >
                {entryLabel}
              </span>
              {suffix != null && (
                <span
                  data-testid="legend-entry-total"
                  className="whitespace-nowrap text-[10px] tabular-nums text-muted-foreground"
                >
                  · {suffix}
                </span>
              )}
            </div>
          );
        })}
        {truncated && (
          <span className="text-[10px] italic text-muted-foreground">
            +{categoryMap.size - MAX_CATEGORICAL_ENTRIES} more
          </span>
        )}
      </div>
    </div>
  );
}
