// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createFileRoute, redirect } from '@tanstack/react-router';
import { decodeViewState } from '@/features/view-state/viewState.codec';
import { readViewStateFields } from '@/features/view-state/viewState.fields';
import { applicationViewStateRepository } from '@/features/view-state/viewState.repository';

export const Route = createFileRoute('/profile/engine/$engineId/query/$queryId/')({
  beforeLoad: async ({ params, search }) => {
    // Reopen the query on the tab the user last left it on.
    const stored =
      search.s === undefined ? await applicationViewStateRepository.load(params) : null;
    const decoded = stored ? decodeViewState(stored) : null;
    const route = decoded?.ok ? readViewStateFields(decoded.value).route : undefined;
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
