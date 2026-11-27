// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { ComponentType, ReactNode } from 'react';
import { SearchX } from 'lucide-react';
import { cn } from '@quent/utils';

export interface EmptyStateProps {
  title: ReactNode;
  description?: ReactNode;
  icon?: ComponentType<{ className?: string }>;
  /** Optional action, usually a `Button`. */
  action?: ReactNode;
  /** `compact` is for short containers like chart rows; it drops the icon tile. */
  size?: 'default' | 'compact';
  /** Fades the whole state so it doesn't compete with nearby data, e.g. in charts. */
  dimmed?: boolean;
  className?: string;
}

/**
 * Centered placeholder for views that have nothing to show.
 * It fills its parent, so give the parent a height.
 */
export function EmptyState({
  title,
  description,
  icon: Icon = SearchX,
  action,
  size = 'default',
  dimmed = false,
  className,
}: EmptyStateProps) {
  const compact = size === 'compact';
  return (
    <div
      className={cn(
        'flex h-full w-full flex-col items-center justify-center text-center',
        compact ? 'gap-0.5 px-4 py-2' : 'gap-1 p-6',
        dimmed && 'opacity-50',
        className
      )}
    >
      {!compact && (
        <div className="mb-2 flex size-12 items-center justify-center rounded-xl border bg-muted/50">
          <Icon className="size-6 text-muted-foreground" />
        </div>
      )}
      <p className={cn('font-medium text-foreground', compact ? 'text-sm' : 'text-base')}>
        {title}
      </p>
      {description && (
        <p className="max-w-sm text-xs text-muted-foreground sm:text-sm">{description}</p>
      )}
      {action && <div className="mt-3">{action}</div>}
    </div>
  );
}
