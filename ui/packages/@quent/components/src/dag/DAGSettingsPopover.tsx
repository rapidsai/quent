// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../ui/select';
import { Button } from '../ui/button';
import { Popover, PopoverContent, PopoverTrigger } from '../ui/popover';
import { Settings2 } from 'lucide-react';
import {
  useNodeColorPalette,
  useEdgeColorPalette,
} from '@quent/hooks';
import {
  CONTINUOUS_PALETTES,
  continuousColor,
  type ContinuousPaletteName,
} from '@quent/utils';

const paletteEntries = Object.entries(CONTINUOUS_PALETTES) as [
  ContinuousPaletteName,
  { label: string },
][];

interface DAGSettingsPopoverProps {
  /** Whether dark mode is active. Passed explicitly to decouple from ThemeContext. */
  isDark: boolean;
}

/** Popover for selecting node and edge color palettes in the DAG view. */
export const DAGSettingsPopover = ({ isDark }: DAGSettingsPopoverProps) => {
  const [nodePalette, setNodePalette] = useNodeColorPalette();
  const [edgePalette, setEdgePalette] = useEdgeColorPalette();

  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          className="h-5 w-5 text-muted-foreground hover:text-foreground"
        >
          <Settings2 className="h-3.5 w-3.5" />
        </Button>
      </PopoverTrigger>
      <PopoverContent side="bottom" className="w-64 flex flex-col gap-3 p-3 shadow-lg">
        <p className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
          Settings
        </p>
        <div className="grid grid-cols-2 gap-2">
          <div className="flex flex-col gap-1">
            <span className="text-xs text-muted-foreground">Node palette</span>
            <Select
              value={nodePalette}
              onValueChange={v => setNodePalette(v as ContinuousPaletteName)}
            >
              <SelectTrigger className="h-7 text-xs">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {paletteEntries.map(([key, { label }]) => (
                  <SelectItem key={key} value={key} className="text-xs">
                    <div className="flex items-center gap-2">
                      <span
                        className="inline-block h-3 w-3 rounded-sm shrink-0"
                        style={{ background: continuousColor(1, key, isDark) }}
                      />
                      {label}
                    </div>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className="flex flex-col gap-1">
            <span className="text-xs text-muted-foreground">Edge palette</span>
            <Select
              value={edgePalette}
              onValueChange={v => setEdgePalette(v as ContinuousPaletteName)}
            >
              <SelectTrigger className="h-7 text-xs">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {paletteEntries.map(([key, { label }]) => (
                  <SelectItem key={key} value={key} className="text-xs">
                    <div className="flex items-center gap-2">
                      <span
                        className="inline-block h-3 w-3 rounded-sm shrink-0"
                        style={{ background: continuousColor(1, key, isDark) }}
                      />
                      {label}
                    </div>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </div>
      </PopoverContent>
    </Popover>
  );
};
