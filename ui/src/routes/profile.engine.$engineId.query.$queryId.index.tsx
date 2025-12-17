import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/profile/engine/$engineId/query/$queryId/')({
  component: QueryIndex,
});

function QueryIndex() {
  return (
    <div className="flex items-center justify-center h-full min-h-[200px]">
      <p className="text-muted-foreground text-center">Select a node to view its profile</p>
    </div>
  );
}
