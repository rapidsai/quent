import { createRootRoute, Link, Outlet, useRouterState } from '@tanstack/react-router';
import { TanStackRouterDevtools } from '@tanstack/router-devtools';
import { ThemeToggle } from '@/components/ThemeToggle';
import { Button } from '@/components/ui/button';
import {
  NavigationMenu,
  NavigationMenuList,
  NavigationMenuItem,
  NavigationMenuLink,
} from '@/components/ui/navigation-menu';
import { cn } from '@/lib/utils';

function RootComponent() {
  const routerState = useRouterState();
  const currentPath = routerState.location.pathname;

  const isActive = (path: string) => {
    if (path === '/') {
      return currentPath === '/';
    }
    return currentPath.startsWith(path);
  };

  return (
    <>
      <div className="min-h-screen flex flex-col bg-background">
        <nav className="sticky top-0 z-50 w-full border-b bg-card shadow-sm">
          <div className="w-full flex h-16 items-center justify-between px-4 sm:px-6 lg:px-8">
            <div className="flex items-center gap-3">
              <img src="/favicon.png" alt="Engage" className="h-10 w-10 object-contain" />
              <h1 className="text-2xl font-bold text-primary">
                PACHA <span className="font-extralight">UI</span>
              </h1>
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
                          isActive('/') && 'bg-accent text-accent-foreground font-semibold'
                        )}
                      >
                        <Link to="/">Dashboard</Link>
                      </Button>
                    </NavigationMenuLink>
                  </NavigationMenuItem>
                  <NavigationMenuItem>
                    <NavigationMenuLink asChild>
                      <Button
                        variant="ghost"
                        size="sm"
                        asChild
                        className={cn(
                          isActive('/profile') && 'bg-accent text-accent-foreground font-semibold'
                        )}
                      >
                        <Link to="/profile">Profile</Link>
                      </Button>
                    </NavigationMenuLink>
                  </NavigationMenuItem>
                  <NavigationMenuItem>
                    <NavigationMenuLink asChild>
                      <Button
                        variant="ghost"
                        size="sm"
                        asChild
                        className={cn(
                          isActive('/about') && 'bg-accent text-accent-foreground font-semibold'
                        )}
                      >
                        <Link to="/about">About</Link>
                      </Button>
                    </NavigationMenuLink>
                  </NavigationMenuItem>
                </NavigationMenuList>
              </NavigationMenu>
              <ThemeToggle />
            </div>
          </div>
        </nav>
        <main className="flex-1 w-full px-4 py-8 sm:px-6 lg:px-8">
          <Outlet />
        </main>
      </div>
      {import.meta.env.DEV && !import.meta.env.TEST && <TanStackRouterDevtools />}
    </>
  );
}

export const Route = createRootRoute({
  component: RootComponent,
});
