// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createFileRoute, Outlet } from '@tanstack/react-router';

export const Route = createFileRoute('/profile')({
  component: ProfileLayout,
});

function ProfileLayout() {
  return <Outlet />;
}
