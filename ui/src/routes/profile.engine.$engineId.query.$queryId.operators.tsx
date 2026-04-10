// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createFileRoute } from '@tanstack/react-router';
import { OperatorTable } from '@/components/operator-table/OperatorTable';
import { Route as QueryRoute } from './profile.engine.$engineId.query.$queryId';

export const Route = createFileRoute('/profile/engine/$engineId/query/$queryId/operators')({
  component: OperatorsTab,
});

function OperatorsTab() {
  const queryBundle = QueryRoute.useLoaderData();
  return <OperatorTable queryBundle={queryBundle} />;
}
