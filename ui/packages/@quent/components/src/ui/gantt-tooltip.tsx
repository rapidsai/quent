// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { ColorSwatch } from './color-swatch';
import { DataText } from './data-text';
import { PointerTooltipPortal } from './pointer-tooltip-portal';
import type { GanttHover } from '../gantt-chart/hover';

export interface GanttTooltipItem {
  id: string;
  color: string;
  name: string;
  detail?: string;
}

export function GanttTooltipPortal({
  hover,
  items,
}: {
  hover: GanttHover | null;
  items: GanttTooltipItem[];
}) {
  if (!hover || items.length === 0) return null;
  return (
    <PointerTooltipPortal hover={hover}>
      <div className="max-h-[50vh] min-w-40 overflow-y-auto rounded bg-popover px-2 py-1.5 text-[11px] leading-tight text-foreground shadow-md">
        <ul className="space-y-1">
          {items.map(item => (
            <li key={item.id} className="flex min-w-0 items-center gap-1.5">
              <ColorSwatch color={item.color} />
              <DataText className="min-w-0 truncate">{item.name}</DataText>
              {item.detail && (
                <DataText className="ml-auto shrink-0 text-muted-foreground">
                  {item.detail}
                </DataText>
              )}
            </li>
          ))}
        </ul>
      </div>
    </PointerTooltipPortal>
  );
}
