# Stack Research

**Domain:** pnpm workspace internal package modularization (React/TypeScript monorepo)
**Researched:** 2026-04-01
**Confidence:** MEDIUM — Web/registry access was unavailable; version numbers drawn from training data (August 2025 cutoff). Verify tsup and unbuild versions before pinning in package.json.

## Context: What This Research Covers

The existing `ui/` app already has React 19, TypeScript 5.9, Vite 7, Vitest 4, Tailwind v4, Jotai, TanStack Query 5, TanStack Router, XYFlow, ECharts, Radix UI, pnpm 9.15. This research covers **only the packaging/monorepo tooling layer** needed to extract code into `ui/packages/@quent/*` workspace packages.

---

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| pnpm workspaces | 9.15.0 (existing) | Internal package linking | Already in use; `workspace:*` protocol gives zero-overhead symlinking with no separate install step. No additional setup beyond a `pnpm-workspace.yaml` in `ui/`. |
| TypeScript project references | 5.9 (existing) | Cross-package type checking | `composite: true` + `references` in `tsconfig.json` gives incremental type-checking across packages without building. The app's existing `tsconfig.node.json` already uses `composite: true`, establishing the pattern. |
| tsup | ^8.x | Package build (for eventual publishing) | esbuild-based, produces ESM + CJS + `.d.ts` in one command with zero config for the common case. Understands `exports` field in `package.json`. Outperforms Vite lib mode for library packages because it doesn't need a browser-oriented pipeline. Widely adopted as the de-facto standard for TypeScript package builds (used by Jotai, TanStack Query, and most major TS libraries themselves). |

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `@arethetypeswrong/cli` | ^0.17.x | Validate `exports` field correctness before publishing | Run as a pre-publish check; catches dual-package hazard, missing CJS/ESM types, wrong `moduleResolution` assumptions. Use in CI once publishing is imminent. |
| `publint` | ^0.2.x | Lint `package.json` for npm publishing correctness | Validates `exports`, `main`, `types` fields. Fast, integrates into `package.json` scripts. Use from the start so publish-readiness issues surface early. |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| `pnpm-workspace.yaml` (in `ui/`) | Declares which directories are workspace packages | One line: `packages: ['packages/*']`. Keeps frontend workspace self-contained — the Rust Cargo workspace at repo root is unaffected. |
| `tsconfig.base.json` (in `ui/`) | Shared compiler options inherited by all packages | Centralises `target`, `lib`, `strict`, `moduleResolution: "bundler"`, `jsx: "react-jsx"`. Packages extend this and add only their `include` / `references`. Prevents drift between package tsconfigs. |
| Vitest (existing) | Per-package unit tests | Each package gets its own `vitest.config.ts` extending the root config. Vitest supports workspace mode (`vitest.workspace.ts`) to run all packages in one command. |

---

## Package Structure Pattern

Each package at `ui/packages/@quent/<name>/` follows this layout:

```
ui/packages/@quent/utils/
  package.json          # name: "@quent/utils", exports, scripts
  tsconfig.json         # extends: "../../tsconfig.base.json"
  src/
    index.ts            # barrel export — complete public API
  dist/                 # built output (gitignored)
```

**`package.json` exports field (publishability pattern):**

```json
{
  "name": "@quent/utils",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "exports": {
    ".": {
      "import": "./dist/index.js",
      "require": "./dist/index.cjs",
      "types": "./dist/index.d.ts"
    }
  },
  "main": "./dist/index.cjs",
  "module": "./dist/index.js",
  "types": "./dist/index.d.ts",
  "scripts": {
    "build": "tsup src/index.ts --format esm,cjs --dts",
    "dev": "tsup src/index.ts --format esm,cjs --dts --watch",
    "typecheck": "tsc --noEmit"
  }
}
```

**Internal consumer `package.json` (app or other package):**

```json
{
  "dependencies": {
    "@quent/utils": "workspace:*",
    "@quent/hooks": "workspace:*"
  }
}
```

`workspace:*` resolves to the local package without version matching. pnpm symlinks the package into `node_modules` transparently.

---

## tsconfig Setup for the Workspace

**`ui/tsconfig.base.json`** — new file, shared base:

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "moduleResolution": "bundler",
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "isolatedModules": true,
    "skipLibCheck": true,
    "resolveJsonModule": true
  }
}
```

**`ui/packages/@quent/<name>/tsconfig.json`** — per-package:

```json
{
  "extends": "../../tsconfig.base.json",
  "compilerOptions": {
    "composite": true,
    "outDir": "./dist",
    "rootDir": "./src",
    "declaration": true,
    "declarationMap": true,
    "noEmit": false
  },
  "include": ["src"]
}
```

Note: package tsconfigs set `noEmit: false` and `outDir: ./dist` because tsup reads these when generating `.d.ts` files. The app's root `tsconfig.json` keeps `noEmit: true` (Vite handles transpilation) and adds `references` pointing to each package.

**`ui/tsconfig.json`** additions (to enable cross-package type checking):

```json
{
  "references": [
    { "path": "./packages/@quent/utils" },
    { "path": "./packages/@quent/hooks" },
    { "path": "./packages/@quent/client" },
    { "path": "./packages/@quent/components" }
  ]
}
```

---

## During Development vs. At Build Time

**During development** (`pnpm dev` in the app): Vite resolves `@quent/*` imports directly to the `src/index.ts` source files through pnpm's symlinks. No package build step is needed for the dev loop. This is the key advantage of the `workspace:*` pattern — Vite's bundler sees TypeScript source directly.

**For CI / publishing**: Each package runs `tsup` to emit `dist/`. The app's production build (`vite build`) also goes through these `dist/` files if `exports` points there, but you can configure Vite to resolve sources in dev via `optimizeDeps.exclude` or leave it as-is since Vite handles `.ts` files natively.

**Recommended approach**: Point `exports` to `dist/` in `package.json` (publish-ready), but configure Vite's workspace-aware alias to resolve to `src/` during development via the `resolve.alias` in `vite.config.ts` or by using the pnpm workspace symlink (Vite picks up `exports["."]["import"]` by default, which points to dist). In practice, most teams skip the Vite alias and just keep `tsup --watch` running alongside `vite dev` — a `pnpm dev` script at the `ui/` level can start both concurrently.

---

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| tsup | Vite lib mode | If the packages need Vite-specific plugins (e.g., Tailwind v4 CSS processing inside the package). For pure TS/TSX packages, tsup is simpler and faster. |
| tsup | unbuild | unbuild (used by Nuxt/UnJS ecosystem) is excellent but more config-heavy for this use case. Better choice if you need rollup plugin ecosystem. tsup covers the JSX + dts case out of the box. |
| tsup | tsc-only (no bundling) | Valid for `@quent/utils` and `@quent/client` if they have zero CSS and no JSX (the app's Vite bundler handles tree-shaking). Breaks down for `@quent/components` which has JSX and Tailwind CSS class usage. Not recommended for consistency across all four packages. |
| tsup | Rollup directly | Rollup is what tsup uses under the hood. Only go direct if tsup's abstraction becomes a constraint — unlikely for this use case. |
| `workspace:*` version | `workspace:^` | `workspace:^` rewrites to a semver range on publish. `workspace:*` rewrites to the exact version. Both are fine; `workspace:*` is more common for internal-only packages that won't publish independently. |

---

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| Lerna | Heavyweight, primarily a publish orchestrator; pnpm native workspaces handle linking natively without it | pnpm workspaces |
| Nx / Turborepo | Valid for large monorepos with many teams but adds significant config surface area and a learning curve that's disproportionate for 4 packages in a single frontend workspace | pnpm `--filter` scripts + Vitest workspace mode |
| Separate `tsconfig.json` per package with full compiler options (no base) | Options drift between packages; a future strict-mode bump requires 4 edits instead of 1 | Shared `tsconfig.base.json` with `extends` |
| Publishing packages to a private npm registry during this milestone | Premature — adds versioning, changelog, and CI complexity before the package boundaries are even stable | `workspace:*` internally; design for publishability, defer execution |
| CJS-only output from tsup | Consumers (including Vite) prefer ESM. CJS-only breaks `import` semantics in strict ESM environments | ESM primary (`"type": "module"`) + CJS secondary via tsup `--format esm,cjs` |
| Top-level `packages/` directory (outside `ui/`) | Would enter the Rust/Cargo workspace context and require cross-workspace config; PROJECT.md explicitly scopes packages to `ui/packages/*` | `ui/packages/@quent/*` |

---

## Version Compatibility

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| tsup ^8.x | TypeScript 5.x, React 19, esbuild 0.24+ | tsup 8.x uses esbuild for transpilation and rollup for bundling. Works with JSX transform (`react-jsx`). `--dts` flag invokes `tsc` for declaration generation, so TypeScript version must match the package's tsconfig. |
| pnpm workspace:* | pnpm 9.x | `workspace:*` protocol is stable since pnpm 6; pnpm 9 is the current major. lockfileVersion 9.0 (confirmed in `ui/pnpm-lock.yaml`) is fully compatible. |
| TypeScript project references | TypeScript 5.x | `composite: true` requires `declaration: true` and `outDir` set. The package tsconfig sets both; the app tsconfig keeps `noEmit: true` and uses references only for type-checking, not emit. |
| Vitest workspace mode | Vitest 4.x (existing) | `vitest.workspace.ts` at `ui/` level references each package's config. Compatible with existing Vitest 4 setup. |

---

## Installation

```bash
# In ui/ — create workspace config
# (create ui/pnpm-workspace.yaml manually)

# In each package — per-package build tooling
cd ui/packages/@quent/utils
pnpm add -D tsup typescript

# Optional: publish linting (run from ui/ level)
pnpm add -D -w publint @arethetypeswrong/cli
```

No new runtime dependencies are needed. All packages reuse `react`, `jotai`, `@tanstack/react-query`, etc. via pnpm's workspace hoisting — these remain as `devDependencies` or `peerDependencies` in each package's `package.json` rather than being re-installed.

---

## Stack Patterns by Variant

**For `@quent/utils` (no JSX, no CSS):**
- `tsup src/index.ts --format esm,cjs --dts --target es2020` is sufficient
- Alternatively, `tsc --emitDeclarationOnly` + `tsc` emit works fine here, but tsup keeps it consistent with the other packages

**For `@quent/hooks` (Jotai atoms, no JSX):**
- Declare `react` and `jotai` as `peerDependencies` (not `dependencies`) so consumers don't get duplicate instances
- tsup with `--external react --external jotai` prevents bundling peers

**For `@quent/client` (TanStack Query hooks, no JSX):**
- Declare `@tanstack/react-query` and `react` as `peerDependencies`
- tsup with `--external react --external @tanstack/react-query`

**For `@quent/components` (JSX + Radix UI + Tailwind CSS classes):**
- Declare `react`, `react-dom`, `@radix-ui/*` as `peerDependencies`
- Tailwind CSS v4 classes are utility strings baked into JSX — no CSS file to bundle from the package. The app's Tailwind build picks them up via `content` glob that includes `ui/packages/**/*.{ts,tsx}`
- tsup `--format esm,cjs --dts --external react --external react-dom`

**If Tailwind CSS classes need to be co-located in the package (content scanning):**
- Add `ui/packages/@quent/components/**/*.{ts,tsx}` to the app's Tailwind `content` array in `vite.config.ts` (or wherever Tailwind v4 scans content)
- No separate CSS build in the package — Tailwind's JIT scans source files at app build time

---

## Sources

- `ui/pnpm-lock.yaml` (lockfileVersion 9.0) — confirmed pnpm 9 workspace compatibility (HIGH confidence)
- `ui/tsconfig.json` — confirmed `moduleResolution: "bundler"`, `noEmit: true`, existing `composite` pattern in `tsconfig.node.json` (HIGH confidence)
- `ui/package.json` — confirmed existing versions: TypeScript 5.9.3, Vite 7.3.1, Vitest 4.0.18 (HIGH confidence)
- Training data (August 2025 cutoff) — tsup version ^8.x, unbuild comparison, pnpm workspace patterns (MEDIUM confidence — verify tsup current version before use)
- `PROJECT.md` — confirmed `ui/packages/*` location requirement, `workspace:*` protocol, publishability design goals (HIGH confidence)

---

*Stack research for: pnpm workspace internal package modularization*
*Researched: 2026-04-01*
