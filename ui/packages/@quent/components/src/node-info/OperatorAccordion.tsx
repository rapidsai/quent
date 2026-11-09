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
  actions,
  children,
}: {
  operator: SelectedOperatorData;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  actions?: ReactNode;
  children: ReactNode;
}) => {
  const locationLabel = operator.locationLabel ?? operator.workerLabel;
  return (
    <DetailsEntry
      open={open}
      onOpenChange={onOpenChange}
      actions={actions}
      testId={`operator-accordion-${operator.nodeId}`}
      bar={
        <OperatorColorBar operationType={operator.operationType} className="w-[3px] self-stretch" />
      }
      triggerLabel={
        locationLabel
          ? `Toggle ${operator.label} (${locationLabel}) details`
          : `Toggle ${operator.label} details`
      }
      header={
        <>
          <DataText className="min-w-0 flex-1 truncate text-xs font-bold" title={operator.label}>
            {operator.label}
          </DataText>
          <DataText className="shrink-0 rounded bg-muted px-1.5 pt-0.5 pb-px text-xs leading-none capitalize text-muted-foreground">
            {operator.operationType}
          </DataText>
          {locationLabel && (
            <DataText
              className="ml-1.5 min-w-0 max-w-[35%] shrink truncate text-[10px] text-muted-foreground"
              title={locationLabel}
            >
              {locationLabel}
            </DataText>
          )}
        </>
      }
    >
      {children}
    </DetailsEntry>
  );
};
