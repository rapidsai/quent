// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { cn, statisticFieldLabel, type StatisticField } from '@quent/utils';

const MAX_CATEGORICAL_ENTRIES = 8;

interface CategoricalLegendProps {
  field: string | StatisticField;
  categoryMap: Map<string, string>;
  dimmedLabels?: ReadonlySet<string>;
  entrySuffixes?: ReadonlyMap<string, string>;
}

export function CategoricalLegend({
  field,
  categoryMap,
  dimmedLabels,
  entrySuffixes,
}: CategoricalLegendProps) {
  const entries = [...categoryMap.entries()].slice(0, MAX_CATEGORICAL_ENTRIES);
  const truncated = categoryMap.size > MAX_CATEGORICAL_ENTRIES;
  const fieldLabel = statisticFieldLabel(field);
  return (
    <div className="flex min-w-0 flex-col gap-1">
      <span
        className="truncate text-[10px] font-semibold uppercase tracking-wide text-muted-foreground"
        title={fieldLabel}
      >
        {fieldLabel}
      </span>
      <div className="flex flex-col gap-0.5">
        {entries.map(([label, color]) => {
          const dimmed = dimmedLabels?.has(label) ?? false;
          const suffix = entrySuffixes?.get(label);
          return (
            <div
              key={label}
              data-dimmed={dimmed || undefined}
              className={cn('flex items-center gap-1.5', dimmed && 'opacity-40')}
            >
              <span
                className="inline-block h-2.5 w-2.5 rounded-sm shrink-0"
                style={{ backgroundColor: color }}
              />
              <span
                className={cn(
                  'text-[10px] text-muted-foreground truncate max-w-[120px]',
                  dimmed && 'line-through'
                )}
                title={label}
              >
                {label}
              </span>
              {suffix != null && (
                <span
                  data-testid="legend-entry-total"
                  className="text-[10px] text-muted-foreground tabular-nums whitespace-nowrap"
                >
                  · {suffix}
                </span>
              )}
            </div>
          );
        })}
        {truncated && (
          <span className="text-[10px] text-muted-foreground italic">
            +{categoryMap.size - MAX_CATEGORICAL_ENTRIES} more
          </span>
        )}
      </div>
    </div>
  );
}
