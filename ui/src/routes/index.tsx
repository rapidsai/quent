import { EngineSelectionPage } from '@/pages/EngineSelectionPage';
import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/')({
  component: EngineSelectionPage,
});
