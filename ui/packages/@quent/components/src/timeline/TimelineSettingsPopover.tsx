// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Settings } from 'lucide-react';
import { Popover, PopoverContent, PopoverTrigger } from '../ui/popover';

export function TimelineSettingsPopover() {
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
      <PopoverContent className="text-xs text-muted-foreground">No settings yet.</PopoverContent>
    </Popover>
  );
}
