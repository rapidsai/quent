// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { ReactNode } from 'react';
import { ChevronDown } from 'lucide-react';
import type { InspectedOperatorData } from '@quent/hooks';
import { DataText } from '../ui/data-text';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '../ui/collapsible';
import { OperatorColorBar } from './OperatorColorBar';

export interface OperatorDisclosureState {
  isOpen: (id: string) => boolean;
  onOpenChange: (id: string, open: boolean) => void;
}

export const OperatorAccordion = ({
  operator,
  open,
  onOpenChange,
  children,
}: {
  operator: InspectedOperatorData;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  children: ReactNode;
}) => (
  <Collapsible
    open={open}
    onOpenChange={onOpenChange}
    className="flex min-w-0 gap-2"
    data-testid={`operator-accordion-${operator.nodeId}`}
  >
    <OperatorColorBar operationType={operator.operationType} className="w-1 self-stretch" />
    <div className="min-w-0 flex-1">
      <CollapsibleTrigger
        className="group flex w-full min-w-0 cursor-pointer items-center gap-2 rounded-sm px-1.5 py-1 my-1 text-left hover:bg-muted/50"
        aria-label={`Toggle ${operator.label} details`}
      >
        <DataText className="min-w-0 truncate text-xs font-medium" title={operator.label}>
          {operator.label}
        </DataText>
        <DataText className="shrink-0 rounded bg-muted px-1.5 py-0.5 text-xs capitalize text-muted-foreground">
          {operator.operationType}
        </DataText>
        <ChevronDown className="ml-auto h-3 w-3 shrink-0 text-muted-foreground transition-transform group-data-[state=closed]:-rotate-90" />
      </CollapsibleTrigger>
      <CollapsibleContent>{children}</CollapsibleContent>
    </div>
  </Collapsible>
);
