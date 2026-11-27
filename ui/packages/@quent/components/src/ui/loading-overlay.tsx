// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useState } from 'react';
import { LoaderCircle } from 'lucide-react';
import { cn } from '@quent/utils';

export interface LoadingOverlayProps {
  label?: string;
  className?: string;
  delayMs?: number;
}

/**
 * Dims the nearest positioned (`relative`) parent and shows a centered loading status.
 * Clicks pass through to the content underneath.
 */
export function LoadingOverlay({ label, className, delayMs = 0 }: LoadingOverlayProps) {
  const [delayElapsed, setDelayElapsed] = useState(false);

  useEffect(() => {
    if (delayMs <= 0) {
      return;
    }
    const timeout = window.setTimeout(() => setDelayElapsed(true), delayMs);
    return () => window.clearTimeout(timeout);
  }, [delayMs]);

  if (delayMs > 0 && !delayElapsed) {
    return null;
  }

  return (
    <div
      className={cn(
        'pointer-events-none absolute inset-0 z-20 flex items-center justify-center bg-background/70 backdrop-blur-[1px]',
        className
      )}
    >
      <span
        role="status"
        aria-live="polite"
        aria-label={label ? undefined : 'Loading'}
        className={cn(
          'flex items-center rounded-lg border bg-card text-sm font-medium text-foreground shadow-lg',
          label ? 'gap-2 px-4 py-2.5' : 'p-2.5'
        )}
      >
        <LoaderCircle className="size-4 animate-spin text-primary" aria-hidden="true" />
        {label}
      </span>
    </div>
  );
}
