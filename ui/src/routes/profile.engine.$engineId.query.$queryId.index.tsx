// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createFileRoute, redirect } from '@tanstack/react-router';

export const Route = createFileRoute('/profile/engine/$engineId/query/$queryId/')({
  beforeLoad: ({ params }) => {
    throw redirect({
      to: '/profile/engine/$engineId/query/$queryId/timeline',
      params,
    });
  },
});
