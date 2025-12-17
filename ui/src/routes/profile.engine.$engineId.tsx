import { createFileRoute, Outlet, useMatch } from '@tanstack/react-router';
import { QueryPlan } from '@/components/QueryPlan';

export const Route = createFileRoute('/profile/engine/$engineId')({
  component: ProfileLayout,
});

function ProfileLayout() {
  const { engineId } = Route.useParams();

  // Type-safe optional match for child route with queryId
  const queryMatch = useMatch({
    from: '/profile/engine/$engineId/query/$queryId/',
    shouldThrow: false,
  });
  const queryId = queryMatch?.params.queryId;

  return (
    <div className="grid grid-cols-[1fr_2fr] gap-6 h-full">
      <div className="border-r pr-6">
        {queryId && queryId !== '' ? (
          <QueryPlan queryId={queryId} engineId={engineId} />
        ) : (
          <div className="flex items-center justify-center h-full text-muted-foreground">
            Select a query to view the execution plan
          </div>
        )}
      </div>
      <div>
        <Outlet />
      </div>
    </div>
  );
}
