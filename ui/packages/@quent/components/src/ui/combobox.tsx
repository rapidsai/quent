// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import * as React from 'react';
import { Check, ChevronsUpDown, X } from 'lucide-react';

import { cn } from '@quent/utils';
import { Button } from './button';
import { Popover, PopoverContent, PopoverTrigger } from './popover';
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from './command';

export interface ComboboxOption {
  value: string;
  label?: string;
}

export interface ComboboxProps {
  options: ComboboxOption[];
  value: string;
  onValueChange: (value: string) => void;
  placeholder?: string;
  searchPlaceholder?: string;
  emptyText?: string;
  disabled?: boolean;
  clearable?: boolean;
  className?: string;
  triggerClassName?: string;
  contentClassName?: string;
  'aria-label'?: string;
}

/**
 * Typeahead single-select built on cmdk + Popover. Filtering is client-side
 * over `options`; swap in a server-driven `options` list to make it a remote
 * typeahead without changing the call sites.
 */
export function Combobox({
  options,
  value,
  onValueChange,
  placeholder = 'Select…',
  searchPlaceholder = 'Search…',
  emptyText = 'No results.',
  disabled,
  clearable = true,
  className,
  triggerClassName,
  contentClassName,
  'aria-label': ariaLabel,
}: ComboboxProps) {
  const [open, setOpen] = React.useState(false);
  const selected = options.find(o => o.value === value);

  return (
    <div className={cn('relative', className)}>
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <Button
            type="button"
            variant="outline"
            role="combobox"
            aria-expanded={open}
            aria-label={ariaLabel}
            disabled={disabled}
            className={cn('w-full justify-between font-normal', triggerClassName)}
          >
            <span className={cn('truncate', !selected && 'text-muted-foreground')}>
              {selected ? (selected.label ?? selected.value) : placeholder}
            </span>
            {clearable && value ? (
              <span
                role="button"
                aria-label="Clear selection"
                className="ml-2 shrink-0 text-muted-foreground hover:text-foreground"
                onPointerDown={e => {
                  e.stopPropagation();
                  e.preventDefault();
                }}
                onClick={e => {
                  e.stopPropagation();
                  onValueChange('');
                }}
              >
                <X className="size-4" />
              </span>
            ) : (
              <ChevronsUpDown className="ml-2 size-4 shrink-0 opacity-50" />
            )}
          </Button>
        </PopoverTrigger>
        <PopoverContent
          align="start"
          className={cn('w-[--radix-popover-trigger-width] p-0', contentClassName)}
        >
          <Command>
            <CommandInput placeholder={searchPlaceholder} />
            <CommandList>
              <CommandEmpty>{emptyText}</CommandEmpty>
              <CommandGroup>
                {options.map(option => (
                  <CommandItem
                    key={option.value}
                    value={`${option.label ?? ''} ${option.value}`}
                    onSelect={() => {
                      onValueChange(option.value === value ? '' : option.value);
                      setOpen(false);
                    }}
                  >
                    <Check
                      className={cn('size-4', option.value === value ? 'opacity-100' : 'opacity-0')}
                    />
                    <span className="truncate">{option.label ?? option.value}</span>
                  </CommandItem>
                ))}
              </CommandGroup>
            </CommandList>
          </Command>
        </PopoverContent>
      </Popover>
    </div>
  );
}
