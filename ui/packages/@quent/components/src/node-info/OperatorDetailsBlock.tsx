// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { InspectedNodeData } from '@quent/hooks';
import type { QuantitySpec } from '@quent/utils';
import { OperatorAccordion, type OperatorDisclosureState } from './OperatorAccordion';
import { OperatorStatFields } from './OperatorStatFields';

export const OperatorDetailsBlock = ({
  operator,
  quantitySpecs,
  isOpen,
  onOpenChange,
}: {
  operator: InspectedNodeData;
  quantitySpecs?: { [key: string]: QuantitySpec | undefined };
} & OperatorDisclosureState) => (
  <OperatorAccordion
    operator={operator}
    open={isOpen(operator.nodeId)}
    onOpenChange={open => onOpenChange(operator.nodeId, open)}
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
