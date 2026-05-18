// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createRootRoute, Link, Outlet, useRouterState } from '@tanstack/react-router';
import { TanStackRouterDevtools } from '@tanstack/react-router-devtools';
import { ThemeProvider } from '@/contexts/ThemeContext';
import { ThemeToggle } from '@/components/ThemeToggle';
import { NavBarNavigator } from '@/components/NavBarNavigator';
import { Button } from '@quent/components';
import {
  NavigationMenu,
  NavigationMenuList,
  NavigationMenuItem,
  NavigationMenuLink,
} from '@quent/components';
import { cn } from '@quent/utils';

function AppNav({ highlightProfile }: { highlightProfile?: boolean }) {
  return (
    <nav className="sticky top-0 z-50 w-full border-b bg-card shadow-sm">
      <div className="w-full flex h-16 items-center justify-between px-4 sm:px-6 lg:px-8">
        <div className="flex items-center gap-3">
          <h1 className="text-2xl font-semibold text-primary">
            QUENT <span className="font-light text-muted-foreground">UI</span>
          </h1>
        </div>
        <div className="flex-1 flex items-center justify-center">
          <NavBarNavigator />
        </div>
        <div className="flex items-center gap-2">
          <NavigationMenu>
            <NavigationMenuList>
              <NavigationMenuItem>
                <NavigationMenuLink asChild>
                  <Button
                    variant="ghost"
                    size="sm"
                    asChild
                    className={cn(
                      highlightProfile && 'bg-accent text-accent-foreground font-semibold'
                    )}
                  >
                    <Link to="/profile">Profile</Link>
                  </Button>
                </NavigationMenuLink>
              </NavigationMenuItem>
            </NavigationMenuList>
          </NavigationMenu>
          <ThemeToggle />
        </div>
      </div>
    </nav>
  );
}

function RootErrorComponent({ error }: { error: Error }) {
  const message = error.message || 'An unexpected error occurred.';
  return (
    <div className="flex flex-col items-center justify-center h-full min-h-64 gap-4 p-8 text-center">
      <div className="space-y-1">
        <h2 className="text-base font-semibold text-destructive">Something went wrong</h2>
        <p className="text-sm text-muted-foreground">{message}</p>
      </div>
      <Button variant="outline" size="sm" asChild>
        <Link to="/profile">Go to profile</Link>
      </Button>
    </div>
  );
}

function RootNotFoundComponent() {
  return (
    <div className="flex flex-col items-center justify-center h-full min-h-64 gap-4 p-8 text-center">
      <div className="space-y-1">
        <h2 className="text-base font-semibold">Page not found</h2>
        <p className="text-sm text-muted-foreground">
          The page you&#39;re looking for doesn&#39;t exist.
        </p>
      </div>
      <Button variant="outline" size="sm" asChild>
        <Link to="/profile">Go to profile</Link>
      </Button>
    </div>
  );
}

function RootComponent() {
  const routerState = useRouterState();
  const isProfileActive = routerState.location.pathname.startsWith('/profile');

  return (
    <>
      <ThemeProvider>
        <div className="min-h-screen flex flex-col bg-background">
          <AppNav highlightProfile={isProfileActive} />
          <main className="flex-1 w-full">
            <Outlet />
          </main>
        </div>
      </ThemeProvider>
      {import.meta.env.VITE_DEBUG && !import.meta.env.TEST && <TanStackRouterDevtools />}
    </>
  );
}

export const Route = createRootRoute({
  component: RootComponent,
  errorComponent: RootErrorComponent,
  notFoundComponent: RootNotFoundComponent,
});
