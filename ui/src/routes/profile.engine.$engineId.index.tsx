import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/profile/engine/$engineId/')({
  component: ProfileIndex,
});

function ProfileIndex() {
  return (
    <div className="flex items-center justify-center h-full min-h-[200px]">
      <p className="text-muted-foreground text-center">
        Enter a query ID and select a node to view profile
      </p>
    </div>
  );
}
