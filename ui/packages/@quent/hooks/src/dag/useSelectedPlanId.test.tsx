// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { PropsWithChildren } from 'react';
import { act, renderHook } from '@testing-library/react';
import { Provider, createStore } from 'jotai';
import { expect, it } from 'vite-plus/test';
import { selectedPlanIdAtom } from '../atoms/dag';
import { useHydrateSelectedPlanId, useSelectedPlanId } from './useSelectedPlanId';

it('initializes the selected plan without replacing a later selection', () => {
  const store = createStore();
  const wrapper = ({ children }: PropsWithChildren) => (
    <Provider store={store}>{children}</Provider>
  );
  const { result, rerender } = renderHook(
    () => {
      useHydrateSelectedPlanId('root-plan');
      return useSelectedPlanId();
    },
    { wrapper }
  );

  expect(result.current).toBe('root-plan');

  act(() => store.set(selectedPlanIdAtom, 'linked-plan'));
  rerender();

  expect(result.current).toBe('linked-plan');
});
