import { createFileRoute } from '@tanstack/react-router';
import { QueryResourceTree } from '@/components/QueryResourceTree';
import { Route as QueryRoute } from './profile.engine.$engineId.query.$queryId';

export const Route = createFileRoute('/profile/engine/$engineId/query/$queryId/timeline')({
  component: TimelineTab,
});

function TimelineTab() {
  const { engineId } = Route.useParams();
  const queryBundle = QueryRoute.useLoaderData();
  return (
    <div className="flex items-center justify-center w-full h-full min-h-[200px]">
      <QueryResourceTree engineId={engineId} queryBundle={queryBundle} />
    </div>
  );
}
