// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { ReactNode } from 'react';
import { ChevronDown } from 'lucide-react';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '../ui/collapsible';

export interface DetailsEntryProps {
  /** The colored bar shown down the left side of the entry. */
  bar: ReactNode;
  /** Content of the clickable header row, shown before the chevron. */
  header: ReactNode;
  actions?: ReactNode;
  triggerLabel: string;
  /** Controlled open state; the entry manages its own state when omitted. */
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  testId?: string;
  children: ReactNode;
}

/** A collapsible entry with a colored bar on its left, shared by the operator and pipe tabs. */
export const DetailsEntry = ({
  bar,
  header,
  actions,
  triggerLabel,
  open,
  onOpenChange,
  testId,
  children,
}: DetailsEntryProps) => (
  <Collapsible
    {...(open === undefined ? { defaultOpen: true } : { open })}
    onOpenChange={onOpenChange}
    className="flex min-w-0 gap-3.5"
    data-testid={testId}
  >
    {bar}
    <div className="min-w-0 flex-1">
      <div className="my-1 flex min-w-0 items-center gap-1">
        <CollapsibleTrigger
          className="group flex min-w-0 flex-1 cursor-pointer items-center gap-2 rounded-sm px-1 py-1 text-left hover:bg-muted/50"
          aria-label={triggerLabel}
        >
          {header}
          <ChevronDown className="h-3 w-3 shrink-0 text-muted-foreground transition-transform group-data-[state=closed]:-rotate-90" />
        </CollapsibleTrigger>
        {actions}
      </div>
      <CollapsibleContent>{children}</CollapsibleContent>
    </div>
  </Collapsible>
);
