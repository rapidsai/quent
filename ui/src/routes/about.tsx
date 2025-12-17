import { createFileRoute } from '@tanstack/react-router';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';

export const Route = createFileRoute('/about')({
  component: About,
});

function About() {
  return (
    <div className="max-w-3xl mx-auto">
      <Card>
        <CardHeader>
          <CardTitle className="text-3xl">About PACHA</CardTitle>
          <CardDescription>
            A modern dashboard application for data visualization and analytics
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <p className="text-muted-foreground leading-relaxed">
            This is a modern dashboard application built with React, TanStack Router, TanStack
            Query, and ECharts for data visualization.
          </p>
          <div className="space-y-2">
            <h3 className="text-lg font-semibold">Technologies Used</h3>
            <ul className="list-disc list-inside space-y-1 text-muted-foreground">
              <li>React 18 with TypeScript</li>
              <li>Vite for fast development and building</li>
              <li>TanStack Router for type-safe routing</li>
              <li>TanStack Query for data fetching and caching</li>
              <li>ECharts for interactive data visualizations</li>
              <li>Tailwind CSS for utility-first styling</li>
              <li>shadcn/ui for beautiful, accessible components</li>
            </ul>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
