// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useLayoutEffect } from 'react';
import { toast } from '@quent/components';
import { DEEP_LINK_SEARCH_KEY } from './deepLink.codec';
import { useDeepLink } from './deepLink.context';
import { showDeepLinkErrorToasts } from './deepLink.toasts';

export function UrlDeepLinkState({ active }: { active: boolean }) {
  const deepLink = useDeepLink();

  useLayoutEffect(() => {
    if (!active) {
      return;
    }
    const url = new URL(window.location.href);
    url.searchParams.delete(DEEP_LINK_SEARCH_KEY);
    window.history.replaceState(
      window.history.state,
      '',
      `${url.pathname}${url.search}${url.hash}`
    );
  }, [active]);

  useEffect(() => {
    if (
      !active ||
      (deepLink?.intakeStatus.kind !== 'error' && deepLink?.intakeStatus.kind !== 'warning')
    ) {
      return;
    }
    const status = deepLink.intakeStatus;
    if (status.kind === 'error') {
      showDeepLinkErrorToasts({
        id: 'deep-link-intake',
        title: 'Could not restore shared view',
        messages: status.messages,
      });
    } else {
      toast.add({
        id: 'deep-link-intake',
        type: 'warning',
        title: 'Shared view adjusted',
        description: status.message,
      });
    }
  }, [active, deepLink?.intakeStatus]);

  return null;
}
