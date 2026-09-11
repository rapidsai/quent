// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { type ReactNode, useMemo, useState } from 'react';
import {
  DeepLinkNavTargetContext,
  type DeepLinkNavTargetContextValue,
} from './deepLinkNavTarget.context';

export function DeepLinkNavTargetProvider({ children }: { children: ReactNode }) {
  const [target, setTarget] = useState<HTMLElement | null>(null);
  const value = useMemo<DeepLinkNavTargetContextValue>(() => ({ target, setTarget }), [target]);

  return (
    <DeepLinkNavTargetContext.Provider value={value}>{children}</DeepLinkNavTargetContext.Provider>
  );
}
