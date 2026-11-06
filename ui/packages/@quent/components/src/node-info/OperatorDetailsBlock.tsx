// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { ReactNode } from 'react';
import type { SelectedOperatorGroupData } from '@quent/hooks';
import type { QuantitySpec } from '@quent/utils';
import { OperatorAccordion, type OperatorDisclosureState } from './OperatorAccordion';
import { OperatorStatFields } from './OperatorStatFields';

export const OperatorDetailsBlock = ({
  operator,
  quantitySpecs,
  isOpen,
  onOpenChange,
  actions,
}: {
  operator: SelectedOperatorGroupData;
  quantitySpecs?: { [key: string]: QuantitySpec | undefined };
  actions?: ReactNode;
} & OperatorDisclosureState) => (
  <OperatorAccordion
    operator={operator}
    open={isOpen(operator.nodeId)}
    onOpenChange={open => onOpenChange(operator.nodeId, open)}
    actions={actions}
  >
    <OperatorStatFields operator={operator} quantitySpecs={quantitySpecs} />
    {operator.relatedOperators?.map(related => (
      <div key={related.nodeId} className="mt-1.5 border-t pt-1.5">
        <OperatorAccordion
          operator={related}
          open={isOpen(related.nodeId)}
          onOpenChange={open => onOpenChange(related.nodeId, open)}
        >
          <OperatorStatFields operator={related} quantitySpecs={quantitySpecs} />
        </OperatorAccordion>
      </div>
    ))}
  </OperatorAccordion>
);
