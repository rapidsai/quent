// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { X } from 'lucide-react';
import { cn } from '@quent/utils';
import { OperatorColorBar } from '../node-info/OperatorColorBar';
import { Badge } from '../ui/badge';
import { Button } from '../ui/button';
import { DataText } from '../ui/data-text';

interface OperatorBadgeProps {
  label: string;
  operationType?: string;
  /** Omit for a read-only pill. */
  onRemove?: () => void;
}

/** A chip for an operator, with a color bar for its operation type. Removable when `onRemove` is set. */
export function OperatorBadge({ label, operationType, onRemove }: OperatorBadgeProps) {
  return (
    <Badge
      variant="outline"
      className={cn(
        'h-6 min-w-0 max-w-64 shrink gap-1.5 overflow-visible bg-muted/40 pl-2 text-sm',
        onRemove ? 'pr-1' : 'pr-2'
      )}
      title={label}
    >
      {operationType && <OperatorColorBar operationType={operationType} className="h-4 w-1" />}
      <DataText className="truncate font-medium text-foreground">{label}</DataText>
      {onRemove && (
        <Button
          type="button"
          variant="ghost"
          size="icon-xs"
          onClick={onRemove}
          aria-label={`Remove ${label}`}
          className="opacity-60 hover:opacity-100"
        >
          <X />
        </Button>
      )}
    </Badge>
  );
}
