// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { DataText } from '../ui/data-text';

export interface FsmCapacityTooltipItem {
  id: string;
  label: string;
  value: string;
}

export function FsmCapacityTooltip({
  stateIndex,
  stateName,
  items,
}: {
  stateIndex: number;
  stateName: string;
  items: FsmCapacityTooltipItem[];
}) {
  return (
    <div className="min-w-36 rounded bg-popover px-2 py-1.5 text-[11px] leading-tight text-foreground shadow-md">
      <DataText className="font-semibold text-muted-foreground">
        {stateIndex + 1}. {stateName}
      </DataText>
      <ul className="mt-1 space-y-0.5">
        {items.map(item => (
          <li key={item.id} className="flex min-w-0 items-center gap-3">
            <DataText className="min-w-0 truncate">{item.label}</DataText>
            <DataText className="ml-auto shrink-0 font-semibold">{item.value}</DataText>
          </li>
        ))}
      </ul>
    </div>
  );
}
