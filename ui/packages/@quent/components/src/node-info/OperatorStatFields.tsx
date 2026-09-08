// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { InspectedOperatorData } from '@quent/hooks';
import { formatStatWithQuantity, type QuantitySpec } from '@quent/utils';
import { DataText } from '../ui/data-text';

export const OperatorStatFields = ({
  operator,
  quantitySpecs,
}: {
  operator: InspectedOperatorData;
  quantitySpecs?: { [key: string]: QuantitySpec | undefined };
}) => (
  <>
    <div className="text-xs flex items-center justify-between">
      <DataText className="capitalize">ID:</DataText>
      <DataText className="text-muted-foreground ml-1 truncate">{operator.nodeId}</DataText>
    </div>
    {operator.statistics.map(({ key, value, quantity }) => (
      <div key={key} className="text-xs">
        {Array.isArray(value) ? (
          <div className="flex items-center justify-between gap-0.5">
            <DataText className="capitalize">{key.replace(/_/g, ' ')}:</DataText>
            <div className="ml-2 flex flex-col gap-0.5">
              {value.map((item, i) => (
                <DataText key={i} className="text-muted-foreground whitespace-pre-line">
                  {String(item)}
                </DataText>
              ))}
            </div>
          </div>
        ) : (
          <div className="flex items-center justify-between">
            <DataText className="capitalize">{key.replace(/_/g, ' ')}:</DataText>
            <DataText className="text-muted-foreground ml-1">
              {typeof value === 'number'
                ? formatStatWithQuantity(
                    value,
                    key,
                    quantity && quantitySpecs ? quantitySpecs[quantity] : undefined
                  )
                : String(value)}
            </DataText>
          </div>
        )}
      </div>
    ))}
  </>
);
