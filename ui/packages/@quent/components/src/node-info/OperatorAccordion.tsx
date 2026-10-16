// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { ReactNode } from 'react';
import type { SelectedOperatorData } from '@quent/hooks';
import { DataText } from '../ui/data-text';
import { DetailsEntry } from './DetailsEntry';
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
  operator: SelectedOperatorData;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  children: ReactNode;
}) => (
  <DetailsEntry
    open={open}
    onOpenChange={onOpenChange}
    testId={`operator-accordion-${operator.nodeId}`}
    bar={
      <OperatorColorBar operationType={operator.operationType} className="w-[3px] self-stretch" />
    }
    triggerLabel={
      operator.workerLabel
        ? `Toggle ${operator.label} (${operator.workerLabel}) details`
        : `Toggle ${operator.label} details`
    }
    header={
      <>
        <DataText className="min-w-0 flex-1 truncate text-xs font-medium" title={operator.label}>
          {operator.label}
        </DataText>
        <DataText className="shrink-0 rounded bg-muted px-1.5 pt-0.5 pb-px text-xs leading-none capitalize text-muted-foreground">
          {operator.operationType}
        </DataText>
        {operator.workerLabel && (
          <DataText
            className="ml-1.5 min-w-0 max-w-[35%] shrink truncate text-[10px] text-muted-foreground"
            title={`Worker: ${operator.workerLabel}`}
          >
            {operator.workerLabel}
          </DataText>
        )}
      </>
    }
  >
    {children}
  </DetailsEntry>
);
