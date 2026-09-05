// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  aggregateDataFlowOperatorFrames,
  type DataFlowFrame,
  type DataFlowMeta,
  type InspectedNodeData,
} from '@quent/hooks';
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
} & OperatorDisclosureState) => {
  const relatedOperatorIds = new Set(
    operator.relatedOperators?.map(related => related.nodeId) ?? []
  );
  const relatedFrames = [...relatedOperatorIds].flatMap(operatorId => {
    const relatedFrame = frame.perOperator.get(operatorId);
    return relatedFrame ? [relatedFrame] : [];
  });
  const operatorFrame =
    relatedOperatorIds.size > 0
      ? aggregateDataFlowOperatorFrames(
          relatedFrames,
          meta.stateNames.length,
          meta.decl.dimension_keys.length,
          frame.labelMeasure === frame.measure
        )
      : frame.perOperator.get(operator.nodeId);

  return (
    <OperatorAccordion
      operator={operator}
      open={isOpen(operator.nodeId)}
      onOpenChange={open => onOpenChange(operator.nodeId, open)}
    >
      <DataFlowMatrix meta={meta} frame={frame} operatorFrame={operatorFrame} isDark={isDark} />
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
};
