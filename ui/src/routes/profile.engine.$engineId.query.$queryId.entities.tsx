// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createFileRoute } from '@tanstack/react-router';
import { EntitiesTable } from '@/components/entities-table/EntitiesTable';
import { Route as QueryRoute } from './profile.engine.$engineId.query.$queryId';

export const Route = createFileRoute('/profile/engine/$engineId/query/$queryId/entities')({
  component: EntitiesTab,
});

function EntitiesTab() {
  const { engineId, queryId } = Route.useParams();
  const queryBundle = QueryRoute.useLoaderData();
  return <EntitiesTable engineId={engineId} queryId={queryId} queryBundle={queryBundle} />;
}
