// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createFileRoute, redirect } from '@tanstack/react-router';
import { browserStorage, readViewAutosave } from '@/features/view-autosave';

export const Route = createFileRoute('/profile/engine/$engineId/query/$queryId/')({
  beforeLoad: ({ params, search }) => {
    // Reopen the query on the tab the user last left it on.
    const tab =
      search.s === undefined ? readViewAutosave(browserStorage(), params)?.tab : undefined;
    throw redirect({
      to: `/profile/engine/$engineId/query/$queryId/${tab ?? 'timeline'}`,
      params,
      search,
    });
  },
});
