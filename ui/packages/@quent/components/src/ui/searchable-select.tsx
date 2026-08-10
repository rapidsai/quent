// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo, useState } from 'react';
import { Check, ChevronDown, Search, X } from 'lucide-react';
import { cn } from '@quent/utils';
import { Button } from './button';
import { Input } from './input';
import { Popover, PopoverContent, PopoverTrigger } from './popover';
import type { SelectFieldOption } from './select-field';

export interface SearchableSelectProps {
  label?: string;
  ariaLabel?: string;
  options: SelectFieldOption[];
  value: string | null;
  onValueChange: (value: string | null) => void;
  placeholder: string;
  searchPlaceholder?: string;
  emptyMessage?: string;
  className?: string;
  triggerClassName?: string;
}

export function SearchableSelect({
  label,
  ariaLabel,
  options,
  value,
  onValueChange,
  placeholder,
  searchPlaceholder = `Search ${(ariaLabel ?? label ?? placeholder).toLowerCase()}…`,
  emptyMessage = 'No matches.',
  className,
  triggerClassName,
}: SearchableSelectProps) {
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState('');
  const selected = options.find(option => option.value === value);
  const filteredOptions = useMemo(() => {
    const needle = search.trim().toLowerCase();
    if (!needle) return options;
    return options.filter(option =>
      `${option.label ?? option.value} ${option.value}`.toLowerCase().includes(needle)
    );
  }, [options, search]);

  const select = (nextValue: string | null) => {
    onValueChange(nextValue);
    setOpen(false);
    setSearch('');
  };

  const accessibleLabel = ariaLabel ?? label;

  return (
    <div className={cn('flex min-w-0', label ? 'items-center gap-1.5' : 'flex-1', className)}>
      {label && (
        <span className="text-xs text-muted-foreground shrink-0 whitespace-nowrap">{label}</span>
      )}
      <Popover
        open={open}
        onOpenChange={nextOpen => {
          setOpen(nextOpen);
          if (!nextOpen) setSearch('');
        }}
      >
        <PopoverTrigger asChild>
          <Button
            variant="outline"
            size="sm"
            role="combobox"
            aria-label={accessibleLabel}
            aria-expanded={open}
            className={cn(
              'h-8 min-w-0 flex-1 justify-between gap-2 px-2 font-normal',
              triggerClassName
            )}
          >
            <span className="truncate text-xs">
              {selected?.label ?? selected?.value ?? placeholder}
            </span>
            <ChevronDown className="size-3.5 shrink-0 opacity-70" />
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-64 p-2" align="start" side="bottom">
          <div className="relative mb-2">
            <Search className="absolute left-2 top-1/2 size-3 -translate-y-1/2 text-muted-foreground pointer-events-none" />
            <Input
              type="text"
              autoFocus
              value={search}
              onChange={event => setSearch(event.target.value)}
              placeholder={searchPlaceholder}
              aria-label={`Search ${(accessibleLabel ?? placeholder).toLowerCase()}`}
              className="h-7 pl-7 pr-6 text-xs md:text-xs"
            />
            {search && (
              <button
                type="button"
                aria-label="Clear search"
                onClick={() => setSearch('')}
                className="absolute right-2 top-1/2 -translate-y-1/2 cursor-pointer text-muted-foreground transition-colors hover:text-foreground"
              >
                <X className="size-3" />
              </button>
            )}
          </div>
          <div className="max-h-56 space-y-0.5 overflow-auto" role="listbox" aria-label={accessibleLabel}>
            <Option label={placeholder} selected={value === null} onSelect={() => select(null)} />
            {filteredOptions.map(option => (
              <Option
                key={option.value}
                label={option.label ?? option.value}
                selected={option.value === value}
                onSelect={() => select(option.value)}
              />
            ))}
            {filteredOptions.length === 0 && (
              <p className="py-2 text-center text-xs text-muted-foreground">{emptyMessage}</p>
            )}
          </div>
        </PopoverContent>
      </Popover>
    </div>
  );
}

function Option({
  label,
  selected,
  onSelect,
}: {
  label: string;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      role="option"
      aria-selected={selected}
      onClick={onSelect}
      className={cn(
        'relative flex w-full cursor-pointer select-none items-center gap-2 rounded-sm px-2 py-1 text-xs outline-none',
        'transition-colors hover:bg-accent hover:text-accent-foreground',
        'focus-visible:bg-accent focus-visible:text-accent-foreground'
      )}
    >
      <Check className={cn('size-3.5 shrink-0', !selected && 'opacity-0')} />
      <span className="truncate">{label}</span>
    </button>
  );
}
