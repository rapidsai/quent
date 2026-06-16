// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Select, SelectContent, SelectItem, SelectTrigger } from '../ui/select';
import { CONTINUOUS_PALETTES, continuousColor, type ContinuousPaletteName } from '@quent/utils';

const paletteEntries = Object.entries(CONTINUOUS_PALETTES) as [
  ContinuousPaletteName,
  { label: string },
][];

interface PalettePickerProps {
  value: ContinuousPaletteName;
  onValueChange: (value: ContinuousPaletteName) => void;
  isDark: boolean;
}

/** Compact color-swatch button that opens a palette selector dropdown. */
export const PalettePicker = ({ value, onValueChange, isDark }: PalettePickerProps) => (
  <Select value={value} onValueChange={v => onValueChange(v as ContinuousPaletteName)}>
    <SelectTrigger className="h-6 w-6 shrink-0 p-0 rounded-sm border border-border overflow-hidden [&>svg]:hidden focus:ring-1 focus:ring-ring">
      <span
        className="block w-full h-full"
        style={{ background: continuousColor(1, value, isDark) }}
      />
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
);
