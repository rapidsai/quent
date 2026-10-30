// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { CSSProperties, ReactNode } from 'react';
import { Eye } from 'lucide-react';
import { cn } from '@quent/utils';
import { Popover, PopoverContent, PopoverTrigger } from '../ui/popover';

interface DAGItemActionsProps {
  label: string;
  visible?: boolean;
  divided?: boolean;
  className?: string;
  backgroundColor?: string;
  previewOpen?: boolean;
  onOpenChange?: (open: boolean) => void;
  previewContent: ReactNode;
}

type ActionStyle = CSSProperties & {
  '--dag-action-background': string;
  '--dag-action-hover-background': string;
};

/** Hover action bar shared by operator nodes and pipes. */
export function DAGItemActions({
  label,
  visible = false,
  divided = true,
  className,
  backgroundColor = 'hsl(var(--primary))',
  previewOpen,
  onOpenChange,
  previewContent,
}: DAGItemActionsProps) {
  const actionClass =
    'flex min-w-0 flex-1 cursor-pointer items-center justify-center gap-1 bg-[var(--dag-action-background)] px-1.5 py-1 text-[10px] font-medium text-foreground transition-colors hover:bg-[var(--dag-action-hover-background)] focus-visible:z-10 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring';
  const actionStyle: ActionStyle = {
    '--dag-action-background': `color-mix(in srgb, ${backgroundColor} 14%, transparent)`,
    '--dag-action-hover-background': `color-mix(in srgb, ${backgroundColor} 26%, transparent)`,
  };

  return (
    <div
      role="toolbar"
      aria-label={`Actions for ${label}`}
      className={cn(
        'nodrag nopan flex w-full bg-card/95 opacity-0 shadow-sm backdrop-blur-sm transition-opacity pointer-events-none',
        'group-hover:opacity-100 group-hover:pointer-events-auto group-focus-within:opacity-100 group-focus-within:pointer-events-auto',
        divided && 'divide-x divide-border',
        visible && 'opacity-100 pointer-events-auto',
        className
      )}
      onPointerDown={event => event.stopPropagation()}
      onClick={event => event.stopPropagation()}
      onMouseLeave={event => {
        const activeElement = document.activeElement;
        if (activeElement instanceof HTMLElement && event.currentTarget.contains(activeElement)) {
          activeElement.blur();
        }
      }}
    >
      <Popover open={previewOpen} onOpenChange={onOpenChange}>
        <PopoverTrigger asChild>
          <button
            type="button"
            className={actionClass}
            style={actionStyle}
            aria-label={`Preview details for ${label}`}
          >
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
    </div>
  );
}
