// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useRouter } from '@tanstack/react-router';
import { Button } from '@quent/components';

interface RouteErrorProps {
  error: Error;
}

export function RouteError({ error }: RouteErrorProps) {
  const router = useRouter();
  const message = error.message || 'An unexpected error occurred.';

  return (
    <div className="flex flex-col items-center justify-center h-full min-h-48 gap-4 p-8 text-center">
      <div className="space-y-1">
        <h2 className="text-base font-semibold text-destructive">Something went wrong</h2>
        <p className="text-sm text-muted-foreground">{message}</p>
      </div>
      <Button variant="outline" size="sm" onClick={() => void router.history.back()}>
        Go back
      </Button>
    </div>
  );
}
