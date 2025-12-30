import { createFileRoute, Outlet, useMatch } from '@tanstack/react-router';
import { QueryPlan } from '@/components/QueryPlan';

export const Route = createFileRoute('/profile/engine/$engineId')({
  component: ProfileLayout,
});

function ProfileLayout() {
  const { engineId } = Route.useParams();

  // Try to match either the query index route or the node route
  const queryIndexMatch = useMatch({
    from: '/profile/engine/$engineId/query/$queryId/',
    shouldThrow: false,
  });
  const queryNodeMatch = useMatch({
    from: '/profile/engine/$engineId/query/$queryId/node/$nodeId',
    shouldThrow: false,
  });
  const queryId = queryIndexMatch?.params?.queryId ?? queryNodeMatch?.params?.queryId;

  return (
    <div className="grid grid-cols-[1fr_2fr] gap-6 h-full">
      <div className="border-r">
        {queryId && queryId !== '' ? (
          <QueryPlan queryId={queryId} engineId={engineId} />
        ) : (
          <div className="flex items-center justify-center h-full text-muted-foreground">
            Select a query to view the execution plan
          </div>
        )}
      </div>
      <div className="overflow-y-auto h-[calc(100vh-4rem)]">
        <Outlet />
      </div>
    </div>
  );
}
