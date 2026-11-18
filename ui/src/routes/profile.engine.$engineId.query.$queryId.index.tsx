// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createFileRoute, redirect } from '@tanstack/react-router';
import { decodeDeepLinkState } from '@/features/deep-link/deepLink.codec';
import { readDeepLinkFields } from '@/features/deep-link/deepLink.fields';
import {
  browserLocalStorage,
  readDeepLinkFromLocalStorage,
} from '@/features/deep-link/deepLink.localStorage';

export const Route = createFileRoute('/profile/engine/$engineId/query/$queryId/')({
  beforeLoad: ({ params, search }) => {
    // Reopen the query on the tab the user last left it on.
    const stored =
      search.s === undefined ? readDeepLinkFromLocalStorage(browserLocalStorage(), params) : null;
    const decoded = stored ? decodeDeepLinkState(stored) : null;
    const route = decoded?.ok ? readDeepLinkFields(decoded.value).route : undefined;
    const tab =
      route?.engineId === params.engineId && route.queryId === params.queryId
        ? route.tab
        : undefined;
    throw redirect({
      to: `/profile/engine/$engineId/query/$queryId/${tab ?? 'timeline'}`,
      params,
      search,
    });
  },
});
