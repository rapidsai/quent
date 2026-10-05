// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { X } from 'lucide-react';
import { OperatorColorBar } from '../node-info';
import { Badge } from '../ui/badge';
import { Button } from '../ui/button';
import { DataText } from '../ui/data-text';

interface OperatorBadgeProps {
  label: string;
  operationType?: string;
  onRemove: () => void;
}

/** A removable chip for a selected operator, with a color bar for its operation type. */
export function OperatorBadge({ label, operationType, onRemove }: OperatorBadgeProps) {
  return (
    <Badge
      variant="outline"
      className="h-6 min-w-0 max-w-64 shrink gap-1.5 overflow-visible bg-muted/40 pl-2 pr-1 text-sm"
      title={label}
    >
      {operationType && <OperatorColorBar operationType={operationType} className="h-4 w-1" />}
      <DataText className="truncate font-medium text-foreground">{label}</DataText>
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
    </Badge>
  );
}
