// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { ReactNode } from 'react';
import { Eye, Filter, Pin } from 'lucide-react';
import { cn } from '@quent/utils';
import { Popover, PopoverContent, PopoverTrigger } from '../ui/popover';

interface DAGItemActionsProps {
  label: string;
  pinned: boolean;
  filtered?: boolean;
  visible?: boolean;
  className?: string;
  onOpenChange?: (open: boolean) => void;
  previewContent: ReactNode;
  onTogglePin: () => void;
  onToggleFilter?: () => void;
}

/** Hover action bar shared by operator nodes and pipes. */
export function DAGItemActions({
  label,
  pinned,
  filtered,
  visible = false,
  className,
  onOpenChange,
  previewContent,
  onTogglePin,
  onToggleFilter,
}: DAGItemActionsProps) {
  const actionClass =
    'flex min-w-0 flex-1 cursor-pointer items-center justify-center gap-1 px-1.5 py-1 text-[10px] font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground focus-visible:z-10 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring aria-pressed:bg-primary aria-pressed:text-primary-foreground aria-pressed:hover:bg-primary/90';

  return (
    <div
      role="toolbar"
      aria-label={`Actions for ${label}`}
      className={cn(
        'nodrag nopan flex w-full divide-x divide-border bg-card/95 opacity-0 shadow-sm backdrop-blur-sm transition-opacity pointer-events-none',
        'group-hover:opacity-100 group-hover:pointer-events-auto group-focus-within:opacity-100 group-focus-within:pointer-events-auto',
        visible && 'opacity-100 pointer-events-auto',
        className
      )}
      onPointerDown={event => event.stopPropagation()}
      onClick={event => event.stopPropagation()}
    >
      <Popover onOpenChange={onOpenChange}>
        <PopoverTrigger asChild>
          <button type="button" className={actionClass} aria-label={`Preview details for ${label}`}>
            <Eye className="size-3" />
            <span>Preview</span>
          </button>
        </PopoverTrigger>
        <PopoverContent
          align="center"
          side="bottom"
          className="max-h-[min(70vh,32rem)] w-[28rem] max-w-[calc(100vw-2rem)] overflow-auto p-3"
          onPointerDown={event => event.stopPropagation()}
          onClick={event => event.stopPropagation()}
        >
          {previewContent}
        </PopoverContent>
      </Popover>
      <button
        type="button"
        className={actionClass}
        aria-label={`Pin details for ${label}`}
        aria-pressed={pinned}
        onClick={onTogglePin}
      >
        <Pin className="size-3" />
        <span>Pin</span>
      </button>
      {onToggleFilter && (
        <button
          type="button"
          className={actionClass}
          aria-label={`Filter operator ${label}`}
          aria-pressed={filtered ?? false}
          onClick={onToggleFilter}
        >
          <Filter className="size-3" />
          <span>Filter</span>
        </button>
      )}
    </div>
  );
}
