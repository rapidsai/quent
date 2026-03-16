import { createFileRoute } from '@tanstack/react-router';
import { OperatorTableAdapter } from '@/components/operator-table/OperatorTableAdapter';
import { Route as QueryRoute } from './profile.engine.$engineId.query.$queryId';

export const Route = createFileRoute('/profile/engine/$engineId/query/$queryId/operators')({
  component: OperatorsTab,
});

function OperatorsTab() {
  const queryBundle = QueryRoute.useLoaderData();
  return <OperatorTableAdapter queryBundle={queryBundle} />;
}
