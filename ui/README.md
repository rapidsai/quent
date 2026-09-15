# Quent UI

A front end for query profiling instrumentation

![Quent overview demo](./docs/screenshots/demo.gif)

## Tech Stack

- **React** - UI library
- **TypeScript** - Type safety
- **Vite** - Build tool and dev server
- **TanStack Router** - Type-safe routing
- **TanStack Query** - Data fetching and state management
- **ECharts** - Data visualization
- **echarts-for-react** - React wrapper for ECharts
- **Tailwind CSS** - Utility-first CSS framework
- **shadcn/ui** - Beautiful, accessible component library
- **Radix UI** - Unstyled, accessible component primitives
- **Lucide React** - Beautiful icon library

## Getting Started

### Prerequisites

Install [Pixi](https://pixi.sh), then enter the repository environment:

```bash
pixi shell
```

Pixi provides the required Rust, Node.js, and pnpm versions. Run UI
development and build commands from this environment.

### Installation

1. Clone the repository (or navigate to the project directory)

2. Install the UI dependencies:

```bash
cd ui
pnpm install
```

3. (Optional) Configure environment variables:

Create a `.env.development.local` file in this directory and add the backend
origin that Vite should proxy `/api` requests to:

```bash
VITE_API_TARGET=http://localhost:8080
```

### Development

Start the development server:

```bash
pnpm dev
```

This generates the TypeScript bindings before starting Vite. Run
`pnpm bindings` after changing Rust types while the dev server remains open.

The app will be available at `http://localhost:5173`

The `pnpm start` script is also available as an alias for the Vite development
server, which is what the end-to-end test runner uses.

### Build

Build the production version:

```bash
pnpm build
```

### Preview

Preview the production build:

```bash
pnpm preview
```

### End-to-End Tests

Install Playwright browser binaries if they are not already present:

```bash
pnpm playwright:install
```

Run the Playwright smoke tests:

```bash
pnpm test:e2e
```

Playwright starts the Vite dev server on `http://127.0.0.1:5173` automatically
unless `PLAYWRIGHT_BASE_URL` is set. The E2E global setup also starts
`quent-simulator-server` on `http://127.0.0.1:18080`, runs the deterministic
`quent-query-engine-fixed` emitter into its collector on `127.0.0.1:17836`, and
stores generated event data in `ui/.e2e-data/`.

In CI only, Playwright writes an HTML report and screenshots failures; the
GitHub UI workflow uploads those files when the E2E job fails.

## API Integration

The browser uses `/api` by default. During local development, Vite proxies those
requests to `VITE_API_TARGET`, which can be set in the shell or in
`.env.development.local`. This keeps the requests same-origin and avoids
requiring CORS support from the backend.

### Run against `quent-open`

From the repository root, start `quent-open` without opening its embedded UI:

```bash
pixi run cargo run --package quent-open -- --no-browser local /path/to/artifacts
```

Once its backend is listening, it prints a copyable setting:

```text
UI backend: VITE_API_TARGET=http://127.0.0.1:49152
```

Use that value when starting the development UI:

```bash
VITE_API_TARGET=http://127.0.0.1:49152 pixi run pnpm --dir ui dev
```

To persist it, put the printed assignment in `ui/.env.development.local` and
restart Vite. `quent-open` chooses a free port on each run, so update the value
after restarting it.

`VITE_API_BASE_URL` controls the browser-facing API URL and defaults to `/api`.
Normally it should remain unchanged for local development so requests continue
through the Vite proxy.

## Customization

### Styling

The application uses Tailwind CSS and shadcn/ui for styling. You can customize
the theme by editing the CSS variables in `src/index.css`:

```css
:root {
  --background: 0 0% 100%;
  --foreground: 222.2 84% 4.9%;
  --primary: 222.2 47.4% 11.2%;
  --secondary: 210 40% 96.1%;
  /* ... more variables */
}
```

You can also customize Tailwind's configuration in `tailwind.config.js` to
extend the default theme with custom colors, fonts, spacing, etc.

### Adding shadcn/ui Components

To add more shadcn/ui components to your project:

```bash
pnpm dlx shadcn@latest add [component-name]
```

For example:

```bash
pnpm dlx shadcn@latest add button
pnpm dlx shadcn@latest add dialog
pnpm dlx shadcn@latest add dropdown-menu
```

### Adding New Routes

1. Create a new file in `src/routes/` (e.g., `analytics.tsx`)
2. Define the route using `createFileRoute`
3. TanStack Router will automatically pick up the new route

## Available Scripts

- `pnpm dev` - Start development server
- `pnpm start` - Start development server
- `pnpm bindings` - Generate TypeScript bindings from Rust types
- `pnpm build` - Build the `@quent` packages and application for production
- `pnpm preview` - Preview production build
- `pnpm lint` - Run ESLint
- `pnpm lint:fix` - Fix ESLint errors and format code
- `pnpm format` - Format code with Prettier
- `pnpm test:e2e` - Run Playwright end-to-end tests

## Development Tools

The application can render TanStack devtools for debugging:

- **TanStack Router Devtools** - Visual router debugging
- **TanStack Query Devtools** - Query state inspection

Both are off by default and gated behind the `VITE_DEBUG` env var, so they
won't appear in normal `pnpm dev` sessions or in production builds.

To enable them, start the dev server with `VITE_DEBUG=1`:

```bash
VITE_DEBUG=1 pnpm dev
```

To leave them on for every dev session on your machine, create a
`ui/.env.development.local` file (already gitignored via `*.local`):

```bash
VITE_DEBUG=1
```

Then a plain `pnpm dev` will pick it up. Restart the dev server after changing
env vars; Vite does not hot-reload `import.meta.env` changes.
