// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo, useState } from 'react';
import { Check, ChevronDown, Search, X } from 'lucide-react';
import { cn } from '@quent/utils';
import { Badge } from './badge';
import { Button } from './button';
import { Input } from './input';
import { Popover, PopoverContent, PopoverTrigger } from './popover';

export interface OptionMultiSelectOption {
  value: string;
  label?: string;
  description?: string;
}

interface OptionMultiSelectProps {
  /** Prefix label rendered before the trigger, e.g. "Columns:". Omit when the field already has an external label (e.g. via a wrapping <FieldWrapper>). */
  label?: string;
  /** Accessible label for the trigger button when the visible label is rendered externally. */
  ariaLabel?: string;
  triggerText: string;
  options: OptionMultiSelectOption[];
  selectedOptionIds: Set<string> | null;
  onToggleOption: (optionId: string) => void;
  onSelectAllOptions: () => void;
  onSelectNoOptions: () => void;
  searchPlaceholder?: string;
  emptyMessage?: string;
  noneSelectedText?: string;
  maxVisibleBadges?: number;
  showSelectedBadges?: boolean;
  className?: string;
  triggerClassName?: string;
  /** Render option labels in a monospace font, e.g. for raw field/column identifiers. Defaults to true. */
  monospaceLabels?: boolean;
}

function optionLabel(option: OptionMultiSelectOption): string {
  return option.label ?? option.value;
}

export function OptionMultiSelect({
  label,
  ariaLabel,
  triggerText,
  options,
  selectedOptionIds,
  onToggleOption,
  onSelectAllOptions,
  onSelectNoOptions,
  searchPlaceholder = 'Search options…',
  emptyMessage = 'No options found',
  noneSelectedText = 'None selected',
  maxVisibleBadges = 6,
  showSelectedBadges = true,
  className,
  triggerClassName,
  monospaceLabels = true,
}: OptionMultiSelectProps) {
  const [search, setSearch] = useState('');

  const isSelected = (value: string): boolean =>
    selectedOptionIds ? selectedOptionIds.has(value) : true;

  const selectedOptions = useMemo(
    () =>
      options.filter(option => (selectedOptionIds ? selectedOptionIds.has(option.value) : true)),
    [options, selectedOptionIds]
  );
  const visibleSelectedOptions = selectedOptions.slice(0, maxVisibleBadges);
  const hiddenSelectedCount = Math.max(0, selectedOptions.length - visibleSelectedOptions.length);

  const filteredOptions = useMemo(() => {
    if (!search) {
      return options;
    }
    const needle = search.toLowerCase();
    return options.filter(option =>
      `${optionLabel(option)} ${option.description ?? ''}`.toLowerCase().includes(needle)
    );
  }, [options, search]);

  const triggerLabel =
    selectedOptions.length === 0
      ? triggerText
      : selectedOptions.length === 1
        ? optionLabel(selectedOptions[0])
        : `${selectedOptions.length} selected`;

  return (
    <div
      className={cn(
        'flex items-center gap-1',
        label ? 'px-3 py-1.5 border-t border-border/50' : 'min-w-0',
        className
      )}
    >
      {label && <span className="text-xs text-muted-foreground shrink-0 mr-1">{label}:</span>}
      <Popover
        onOpenChange={open => {
          if (!open) {
            setSearch('');
          }
        }}
      >
        <PopoverTrigger asChild>
          <Button
            variant="outline"
            size="sm"
            role="combobox"
            aria-label={ariaLabel ?? label}
            className={cn(
              'h-7 min-w-36 justify-between gap-2 px-2 text-xs font-normal hover:bg-background hover:text-foreground',
              triggerClassName
            )}
          >
            <span className="flex-1 truncate text-left">{triggerLabel}</span>
            {selectedOptions.length > 0 && (
              <span
                role="button"
                aria-label={`Clear ${ariaLabel ?? label ?? triggerText}`}
                className="shrink-0 cursor-pointer text-muted-foreground transition-colors hover:text-foreground"
                onPointerDown={e => {
                  e.stopPropagation();
                  e.preventDefault();
                }}
                onClick={e => {
                  e.stopPropagation();
                  onSelectNoOptions();
                }}
              >
                <X className="size-3!" />
              </span>
            )}
            <ChevronDown className="text-muted-foreground shrink-0 opacity-70" />
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-64 p-2" align="start" side="bottom">
          <div className="relative mb-2">
            <Search className="absolute left-2 top-1/2 -translate-y-1/2 size-3 text-muted-foreground pointer-events-none" />
            <Input
              type="search"
              className="h-7 pl-7 pr-2 text-xs md:text-xs"
              placeholder={searchPlaceholder}
              aria-label={searchPlaceholder}
              value={search}
              onChange={e => setSearch(e.target.value)}
              autoFocus
            />
          </div>
          <div className="flex gap-1 mb-2 border-b border-border pb-2">
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={onSelectAllOptions}
              className="h-6 px-2 text-xs text-primary hover:text-primary"
            >
              All
            </Button>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={onSelectNoOptions}
              className="h-6 px-2 text-xs text-primary hover:text-primary"
            >
              None
            </Button>
          </div>
          <div role="listbox" aria-multiselectable className="max-h-52 overflow-y-auto space-y-0.5">
            {filteredOptions.map(option => {
              const checked = isSelected(option.value);
              return (
                <button
                  key={option.value}
                  type="button"
                  role="option"
                  aria-selected={checked}
                  data-state={checked ? 'checked' : 'unchecked'}
                  onClick={() => onToggleOption(option.value)}
                  className={cn(
                    'relative flex w-full cursor-pointer select-none items-center gap-2 rounded-sm px-2 py-1 text-xs outline-none',
                    'transition-colors hover:bg-accent hover:text-accent-foreground',
                    'focus-visible:bg-accent focus-visible:text-accent-foreground'
                  )}
                >
                  <span
                    aria-hidden
                    className={cn(
                      'mt-0.5 flex size-3.5 shrink-0 items-center justify-center rounded-sm border transition-colors',
                      checked
                        ? 'bg-primary border-primary text-primary-foreground'
                        : 'border-input bg-background'
                    )}
                  >
                    {checked && <Check className="size-2.5" strokeWidth={3} />}
                  </span>
                  <span className="min-w-0 flex-1 text-left">
                    <span className={cn('block truncate', monospaceLabels && 'font-mono')}>
                      {optionLabel(option)}
                    </span>
                    {option.description && (
                      <span className="block truncate text-[10px] text-muted-foreground">
                        {option.description}
                      </span>
                    )}
                  </span>
                </button>
              );
            })}
            {filteredOptions.length === 0 && (
              <p className="text-xs text-muted-foreground text-center py-2">{emptyMessage}</p>
            )}
          </div>
        </PopoverContent>
      </Popover>
      {showSelectedBadges && (
        <div className="flex-1 min-w-0">
          {selectedOptions.length === 0 ? (
            <span className="text-xs text-muted-foreground italic">{noneSelectedText}</span>
          ) : (
            <div className="flex flex-wrap items-center gap-1">
              {visibleSelectedOptions.map(option => (
                <Badge
                  key={option.value}
                  variant="outline"
                  className={cn(
                    'px-1.5 py-0 text-data bg-primary/10 border-primary/40 hover:bg-primary/15',
                    monospaceLabels && 'font-mono'
                  )}
                >
                  <span className="truncate">{optionLabel(option)}</span>
                  <button
                    type="button"
                    onClick={e => {
                      e.stopPropagation();
                      onToggleOption(option.value);
                    }}
                    aria-label={`Remove ${optionLabel(option)}`}
                    className="ml-0.5 rounded-sm opacity-70 hover:opacity-100 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring cursor-pointer"
                  >
                    <X className="size-2.5" />
                  </button>
                </Badge>
              ))}
              {hiddenSelectedCount > 0 && (
                <Badge variant="outline" className="px-1.5 py-0 bg-muted/40 text-muted-foreground">
                  +{hiddenSelectedCount} more
                </Badge>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
