// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useLayoutEffect } from 'react';
import { toast } from '@quent/components';
import { VIEW_STATE_SEARCH_KEY } from './viewState.codec';
import { useViewState } from './viewState.context';
import { showViewStateErrorToasts } from './viewState.toasts';

export function UrlViewState({ active }: { active: boolean }) {
  const viewState = useViewState();

  useLayoutEffect(() => {
    if (!active) {
      return;
    }
    const url = new URL(window.location.href);
    url.searchParams.delete(VIEW_STATE_SEARCH_KEY);
    window.history.replaceState(
      window.history.state,
      '',
      `${url.pathname}${url.search}${url.hash}`
    );
  }, [active]);

  useEffect(() => {
    if (
      !active ||
      (viewState?.intakeStatus.kind !== 'error' && viewState?.intakeStatus.kind !== 'warning')
    ) {
      return;
    }
    const status = viewState.intakeStatus;
    if (status.kind === 'error') {
      showViewStateErrorToasts({
        id: 'view-state-intake',
        title: 'Could not restore shared view',
        messages: status.messages,
      });
    } else {
      toast.add({
        id: 'view-state-intake',
        type: 'warning',
        title: 'Shared view adjusted',
        description: status.message,
      });
    }
  }, [active, viewState?.intakeStatus]);

  return null;
}
