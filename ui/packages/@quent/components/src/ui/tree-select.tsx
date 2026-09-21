// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo, useState, type ReactNode } from 'react';
import { Check, ChevronDown } from 'lucide-react';
import { cn } from '@quent/utils';
import { Button } from './button';
import { Popover, PopoverContent, PopoverTrigger } from './popover';
import { TreeView, type TreeDataItem, type TreeRenderItemParams } from './tree-view';

export interface TreeSelectProps<T extends TreeDataItem> {
  data: T[] | T;
  value?: string;
  onValueChange: (item: T) => void;
  ariaLabel: string;
  title?: string;
  getItemTitle?: (item: T) => string;
  placeholder?: ReactNode;
  renderItem?: (params: TreeRenderItemParams<T>) => ReactNode;
  renderValue?: (item: T) => ReactNode;
  onItemHover?: (item: T | null) => void;
  disabled?: boolean;
  expandAll?: boolean;
  collapsible?: boolean;
  compact?: boolean;
  className?: string;
  triggerClassName?: string;
  contentClassName?: string;
}

function findTreeItem<T extends TreeDataItem>(data: T[] | T, id: string): T | undefined {
  const items = Array.isArray(data) ? data : [data];
  for (const item of items) {
    if (item.id === id) {
      return item;
    }
    if (item.children) {
      const match = findTreeItem(item.children as T[], id);
      if (match) {
        return match;
      }
    }
  }
  return undefined;
}

/** Selects one item from hierarchical data using a tree inside a popover. */
export function TreeSelect<T extends TreeDataItem>({
  data,
  value,
  onValueChange,
  ariaLabel,
  title,
  getItemTitle,
  placeholder = 'Select an item',
  renderItem,
  renderValue,
  onItemHover,
  disabled,
  expandAll,
  collapsible = true,
  compact = true,
  className,
  triggerClassName,
  contentClassName,
}: TreeSelectProps<T>) {
  const [open, setOpen] = useState(false);
  const selectedItem = useMemo(
    () => (value === undefined ? undefined : findTreeItem(data, value)),
    [data, value]
  );

  const handleOpenChange = (nextOpen: boolean) => {
    setOpen(nextOpen);
    if (!nextOpen) {
      onItemHover?.(null);
    }
  };

  const handleSelect = (item: T | undefined) => {
    if (!item) {
      return;
    }
    onValueChange(item);
    handleOpenChange(false);
  };

  const renderTreeItem = (params: TreeRenderItemParams<T>) => (
    <div
      className="flex min-w-0 flex-1 items-center gap-2"
      title={getItemTitle?.(params.item) ?? params.item.name}
    >
      <div className="min-w-0 flex-1">
        {renderItem?.(params) ?? <span className="block truncate text-xs">{params.item.name}</span>}
      </div>
      {params.isSelected && (
        <Check role="img" aria-label="Selected item" className="size-3.5 shrink-0 text-primary" />
      )}
    </div>
  );

  return (
    <Popover open={open} onOpenChange={handleOpenChange}>
      <PopoverTrigger asChild>
        <Button
          type="button"
          variant="outline"
          role="combobox"
          aria-label={ariaLabel}
          aria-haspopup="tree"
          aria-expanded={open}
          title={selectedItem ? (getItemTitle?.(selectedItem) ?? selectedItem.name) : title}
          disabled={disabled}
          className={cn(
            'h-auto min-h-9 min-w-0 flex-1 justify-between gap-2 px-2 py-1 font-normal whitespace-normal',
            triggerClassName,
            className
          )}
        >
          <span className="min-w-0 flex-1 text-left">
            {selectedItem
              ? (renderValue?.(selectedItem) ?? (
                  <span className="block truncate text-xs">{selectedItem.name}</span>
                ))
              : placeholder}
          </span>
          <ChevronDown
            className={cn(
              'size-3.5 shrink-0 opacity-70 transition-transform',
              open && 'rotate-180'
            )}
          />
        </Button>
      </PopoverTrigger>
      <PopoverContent
        align="start"
        side="bottom"
        className={cn(
          'max-h-72 w-[var(--radix-popover-trigger-width)] overflow-y-auto p-1',
          contentClassName
        )}
      >
        <TreeView<T>
          data={data}
          selectedItemId={value}
          onSelectChange={handleSelect}
          expandAll={expandAll}
          collapsible={collapsible}
          compact={compact}
          renderItem={renderTreeItem}
          onItemHover={onItemHover}
          aria-label={ariaLabel}
        />
      </PopoverContent>
    </Popover>
  );
}
