// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Badge } from '../badge';

export interface ContinuousLegendProps {
  label: string;
  min: number;
  max: number;
  gradientStops: readonly string[];
  formatValue: (value: number) => string;
  scaleLabel: string;
}

export function ContinuousLegend({
  label,
  min,
  max,
  gradientStops,
  formatValue,
  scaleLabel,
}: ContinuousLegendProps) {
  return (
    <div className="flex min-w-0 flex-col gap-1">
      <div className="flex min-w-0 items-center justify-between gap-2">
        <span
          className="min-w-0 flex-1 truncate text-[10px] font-semibold uppercase tracking-wide text-muted-foreground"
          title={label}
        >
          {label}
        </span>
        <Badge
          variant="outline"
          className="pointer-events-none px-1.5 py-0 text-[10px] text-muted-foreground transition-none hover:bg-transparent hover:text-muted-foreground"
        >
          {scaleLabel}
        </Badge>
      </div>
      <div
        className="h-2 w-full rounded-sm"
        style={{ background: `linear-gradient(to right, ${gradientStops.join(', ')})` }}
      />
      <div className="flex justify-between gap-2">
        <span className="text-[10px] text-muted-foreground">{formatValue(min)}</span>
        <span className="text-[10px] text-muted-foreground">{formatValue(max)}</span>
      </div>
    </div>
  );
}
