// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useState } from 'react';
import { Settings } from 'lucide-react';
import { Popover, PopoverContent, PopoverTrigger } from '../ui/popover';
import { DAGControls } from './DAGControls';

interface DAGSettingsPopoverProps {
  operatorStatFields: string[];
  portStatFields: string[];
  isDark: boolean;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
}

export function DAGSettingsPopover({
  operatorStatFields,
  portStatFields,
  isDark,
  open: controlledOpen,
  onOpenChange,
}: DAGSettingsPopoverProps) {
  const [uncontrolledOpen, setUncontrolledOpen] = useState(false);
  const open = controlledOpen ?? uncontrolledOpen;
  const setOpen = (nextOpen: boolean) => {
    if (controlledOpen === undefined) {
      setUncontrolledOpen(nextOpen);
    }
    onOpenChange?.(nextOpen);
  };

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <button
          type="button"
          aria-label="Settings"
          className="mr-1 inline-flex aspect-square min-h-9 self-stretch shrink-0 cursor-pointer items-center justify-center rounded-sm text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
          title="Settings"
        >
          <Settings className="size-4" />
        </button>
      </PopoverTrigger>
      <PopoverContent
        className="max-h-[min(70vh,36rem)] w-[min(36rem,calc(100vw-2rem))] overflow-y-auto p-0"
        onPointerDownOutside={() => setOpen(false)}
      >
        <DAGControls
          operatorStatFields={operatorStatFields}
          portStatFields={portStatFields}
          isDark={isDark}
        />
      </PopoverContent>
    </Popover>
  );
}
