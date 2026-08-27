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
  fields?: { label: string; value: string }[];
}

const TOOLTIP_ITEM_LIMIT = 6;

export function GanttTooltipPortal({
  hover,
  items,
}: {
  hover: GanttHover | null;
  items: GanttTooltipItem[];
}) {
  if (!hover || items.length === 0) {
    return null;
  }
  const visibleItems = items.slice(0, TOOLTIP_ITEM_LIMIT);
  const hiddenCount = items.length - visibleItems.length;
  return (
    <PointerTooltipPortal hover={hover}>
      <div className="max-h-[50vh] min-w-40 overflow-y-auto rounded bg-popover px-2 py-1.5 text-[11px] leading-tight text-foreground shadow-md">
        <ul className="space-y-1">
          {visibleItems.map(item => (
            <li key={item.id} className="flex min-w-0 flex-col gap-0.5">
              <div className="flex min-w-0 items-center gap-1.5">
                <ColorSwatch color={item.color} />
                <DataText className="min-w-0 truncate">{item.name}</DataText>
                {item.detail && (
                  <DataText className="ml-auto shrink-0 text-muted-foreground">
                    {item.detail}
                  </DataText>
                )}
              </div>
              {item.fields?.map(field => (
                <div key={field.label} className="flex gap-2 pl-3.5 text-muted-foreground">
                  <DataText className="shrink-0">{field.label}</DataText>
                  <DataText className="ml-auto min-w-0 truncate">{field.value}</DataText>
                </div>
              ))}
            </li>
          ))}
        </ul>
        {hiddenCount > 0 && (
          <DataText as="div" className="pt-1 text-muted-foreground">
            {hiddenCount} more {hiddenCount === 1 ? 'item' : 'items'} not shown
          </DataText>
        )}
      </div>
    </PointerTooltipPortal>
  );
}
