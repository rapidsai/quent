import { createFileRoute, Outlet, useMatch } from '@tanstack/react-router';
import { Provider } from 'jotai';
import { QueryPlan } from '@/components/QueryPlan';
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from '@/components/ui/resizable';

export const Route = createFileRoute('/profile/engine/$engineId')({
  component: ProfileLayout,
});

function ProfileLayout() {
  const { engineId } = Route.useParams();

  // Match the query layout route (covers all /query/$queryId/* children)
  const queryMatch = useMatch({
    from: '/profile/engine/$engineId/query/$queryId',
    shouldThrow: false,
  });
  const queryId = queryMatch?.params?.queryId;

  return (
    <Provider key={queryId ?? ''}>
      <ResizablePanelGroup orientation="horizontal" className="h-full">
        <ResizablePanel defaultSize="33%" minSize="15%" collapsible collapsedSize="0%">
          {queryId && queryId !== '' ? (
            <QueryPlan queryId={queryId} engineId={engineId} />
          ) : (
            <div className="flex items-center justify-center h-full text-muted-foreground">
              Select a query to view the execution plan
            </div>
          )}
        </ResizablePanel>
        <ResizableHandle withHandle />
        <ResizablePanel
          defaultSize="67%"
          minSize="20%"
          collapsible
          collapsedSize="0%"
          className="overflow-y-auto h-[calc(100vh-4rem)]"
        >
          <Outlet />
        </ResizablePanel>
      </ResizablePanelGroup>
    </Provider>
  );
}
