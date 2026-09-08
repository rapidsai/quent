// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Settings } from 'lucide-react';
import { useId } from 'react';
import {
  LONG_ENTITY_DENSITIES,
  useLongEntityDensity,
  useSetLongEntityDensity,
  type LongEntityDensity,
} from '@quent/hooks';
import { Popover, PopoverContent, PopoverTrigger } from '../ui/popover';

const DENSITY_MIN = Math.min(...LONG_ENTITY_DENSITIES);
const DENSITY_MAX = Math.max(...LONG_ENTITY_DENSITIES);

function isLongEntityDensity(value: number): value is LongEntityDensity {
  return LONG_ENTITY_DENSITIES.some(density => density === value);
}

export function TimelineSettingsPopover() {
  const density = useLongEntityDensity();
  const setDensity = useSetLongEntityDensity();
  const sliderId = useId();

  return (
    <Popover>
      <PopoverTrigger asChild>
        <button
          type="button"
          aria-label="Timeline settings"
          className="inline-flex cursor-pointer items-center rounded-sm p-0.5 transition-colors hover:bg-accent hover:text-accent-foreground"
          title="Timeline settings"
        >
          <Settings className="h-3.5 w-3.5" />
        </button>
      </PopoverTrigger>
      <PopoverContent className="w-56">
        <label htmlFor={sliderId} className="text-xs font-medium text-foreground">
          Entities
        </label>
        <input
          id={sliderId}
          type="range"
          min={DENSITY_MIN}
          max={DENSITY_MAX}
          step={1}
          value={density}
          aria-valuetext={`${density} out of ${DENSITY_MAX}`}
          onChange={event => {
            const nextDensity = Number(event.target.value);
            if (isLongEntityDensity(nextDensity)) {
              setDensity(nextDensity);
            }
          }}
          className="mt-2 h-1.5 w-full cursor-pointer accent-primary"
        />
        <div aria-hidden className="mt-1 flex justify-between text-[10px] text-muted-foreground">
          <span>Less</span>
          <span>More</span>
        </div>
      </PopoverContent>
    </Popover>
  );
}
