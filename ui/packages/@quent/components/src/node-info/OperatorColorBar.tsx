// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { cn, getOperationTypeColor } from '@quent/utils';

export const OperatorColorBar = ({
  operationType,
  className,
}: {
  operationType: string;
  className?: string;
}) => (
  <span
    aria-hidden
    data-testid="operator-color-bar"
    data-operation-type={operationType}
    className={cn('shrink-0 rounded-full', className)}
    style={{ backgroundColor: getOperationTypeColor(operationType) }}
  />
);
