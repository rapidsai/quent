import { createFileRoute, redirect } from '@tanstack/react-router';

export const Route = createFileRoute('/profile/engine/$engineId/query/$queryId/')({
  beforeLoad: ({ params }) => {
    throw redirect({
      to: '/profile/engine/$engineId/query/$queryId/timeline',
      params,
    });
  },
});
