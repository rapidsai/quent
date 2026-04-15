// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { X } from 'lucide-react';
import { cn } from '@quent/utils';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from './select';
import { ScrollArea } from './scroll-area';

export interface SelectFieldOption {
  value: string;
  label?: string;
}

export interface SelectFieldProps {
  /** Optional leading icon */
  icon?: React.ElementType;
  /** Optional label rendered before the trigger */
  label?: string;
  options: SelectFieldOption[];
  value: string;
  onValueChange: (value: string | null) => void;
  placeholder?: string;
  /** Show a clear button inside the trigger when a value is selected */
  clearable?: boolean;
  /** className applied to the outer wrapper */
  className?: string;
  /** className forwarded to SelectTrigger */
  triggerClassName?: string;
}

/** Select dropdown with optional label, icon, and clear button. */
export const SelectField = ({
  icon: Icon,
  label,
  options,
  value,
  onValueChange,
  placeholder,
  clearable = true,
  className,
  triggerClassName,
}: SelectFieldProps) => (
  <div className={cn('flex items-center gap-1.5 min-w-0', className)}>
    {Icon && <Icon className="h-3 w-3 shrink-0 text-muted-foreground" />}
    {label && (
      <span className="text-xs text-muted-foreground shrink-0 whitespace-nowrap">{label}</span>
    )}
    <Select value={value} onValueChange={onValueChange}>
      <SelectTrigger className={cn('flex-1 min-w-0', triggerClassName)}>
        <SelectValue placeholder={placeholder} />
        {clearable && value && (
          <span
            role="button"
            aria-label={`Clear ${label ?? 'selection'}`}
            className="ml-auto mr-1 shrink-0 text-muted-foreground hover:text-foreground transition-colors"
            onPointerDown={e => {
              e.stopPropagation();
              e.preventDefault();
            }}
            onClick={e => {
              e.stopPropagation();
              onValueChange(null);
            }}
          >
            <X className="h-3 w-3" />
          </span>
        )}
      </SelectTrigger>
      <SelectContent>
        <ScrollArea viewportClassName="max-h-[10rem]">
          <SelectGroup>
            {options.length === 0 ? (
              <SelectItem value="_empty" disabled>
                No data available
              </SelectItem>
            ) : (
              options.map(opt => (
                <SelectItem key={opt.value} value={opt.value} className="text-xs">
                  {opt.label ?? opt.value}
                </SelectItem>
              ))
            )}
          </SelectGroup>
        </ScrollArea>
      </SelectContent>
    </Select>
  </div>
);
