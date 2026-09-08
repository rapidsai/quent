// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { DataFlowFrame, DataFlowMeta, InspectedNodeData } from '@quent/hooks';
import { DataFlowMatrix } from '../dag/DataFlowMatrix';
import { OperatorAccordion, type OperatorDisclosureState } from './OperatorAccordion';

export const OperatorDataFlowBlock = ({
  operator,
  meta,
  frame,
  isDark,
  isOpen,
  onOpenChange,
}: {
  operator: InspectedNodeData;
  meta: DataFlowMeta;
  frame: DataFlowFrame;
  isDark: boolean;
} & OperatorDisclosureState) => (
  <OperatorAccordion
    operator={operator}
    open={isOpen(operator.nodeId)}
    onOpenChange={open => onOpenChange(operator.nodeId, open)}
  >
    <DataFlowMatrix
      meta={meta}
      frame={frame}
      operatorFrame={frame.perOperator.get(operator.nodeId)}
      isDark={isDark}
    />
    {operator.relatedOperators?.map(related => (
      <div key={related.nodeId} className="mt-1.5 border-t pt-1.5">
        <OperatorAccordion
          operator={related}
          open={isOpen(related.nodeId)}
          onOpenChange={open => onOpenChange(related.nodeId, open)}
        >
          <DataFlowMatrix
            meta={meta}
            frame={frame}
            operatorFrame={frame.perOperator.get(related.nodeId)}
            isDark={isDark}
          />
        </OperatorAccordion>
      </div>
    ))}
  </OperatorAccordion>
);
