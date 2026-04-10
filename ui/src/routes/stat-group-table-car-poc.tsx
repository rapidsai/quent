// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createFileRoute } from '@tanstack/react-router';
import { CarPriceStatGroupTablePoc } from '@/components/pivot-table/CarPriceStatGroupTablePoc';

export const Route = createFileRoute('/stat-group-table-car-poc')({
  component: StatGroupTableCarPocPage,
});

function StatGroupTableCarPocPage() {
  return (
    <div className="h-[calc(100vh-4rem)] p-4">
      <div className="h-full overflow-hidden rounded-md border border-border">
        <CarPriceStatGroupTablePoc />
      </div>
    </div>
  );
}
