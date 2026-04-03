// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { cn } from '@/lib/utils';

interface InlineSelectorProps {
  id: string;
  label?: string;
  value: string;
  options: string[];
  onChange: (itemId: string, type: string) => void;
  className?: string;
}

export const InlineSelector = ({
  id,
  label = 'Type',
  value,
  options,
  onChange,
  className,
}: InlineSelectorProps): React.ReactNode => {
  return (
    <div
      className={cn('flex items-center gap-1.5', className)}
      onClick={e => e.stopPropagation()}
      onMouseDown={e => e.stopPropagation()}
    >
      <label id={`type-select-label-${id}`} className="text-xs text-muted-foreground shrink-0">
        {label}:
      </label>
      <Select value={value} onValueChange={value => onChange(id, value)}>
        <SelectTrigger
          id={`type-select-${id}`}
          aria-labelledby={`type-select-label-${id}`}
          className={cn(
            'h-auto w-auto min-w-0 max-w-80 border-0 border-b border-dashed border-muted-foreground/60 rounded-none bg-transparent px-0 py-px text-xs font-mono shadow-none cursor-pointer',
            'focus:ring-0 focus:ring-offset-0 focus-visible:ring-0 focus-visible:ring-offset-0',
            'data-[placeholder]:text-muted-foreground',
            '[&>svg]:h-3 [&>svg]:w-3 [&>svg]:shrink-0 [&>svg]:translate-y-px [&>svg]:opacity-70'
          )}
        >
          <SelectValue />
        </SelectTrigger>
        <SelectContent
          position="popper"
          className="max-h-[--radix-select-content-available-height] min-w-[var(--radix-select-trigger-width)]"
        >
          {options.map(option => (
            <SelectItem
              key={option}
              value={option}
              className="text-xs font-mono py-1.5 pl-8 pr-2 cursor-pointer"
            >
              {option}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
};
