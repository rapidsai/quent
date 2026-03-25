# Contributing to Quent

Thank you for your interest in contributing to Quent!

## Issue Tracking

All bug fixes, enhancements, and other changes must begin with the creation of a
[GitHub Issue](https://github.com/NVIDIA/quent/issues). The issue must be
reviewed and approved by a Quent maintainer before code review begins on any
associated pull request.

## Pull Requests

The developer workflow for code contributions is as follows:

1. Fork the upstream Quent repository on GitHub.

2. Clone your fork and create a branch for your changes:

   ```bash
   git clone https://github.com/YOUR_USERNAME/quent.git
   cd quent
   git checkout -b my-feature-branch
   ```

3. Make your changes, ensuring all commits are signed off (see
   [Developer Certificate of Origin](#developer-certificate-of-origin-dco)
   below).

4. Push your branch to your fork:

   ```bash
   git push -u origin my-feature-branch
   ```

5. Open a Pull Request against the `main` branch of the upstream repository.
   - If the PR is not yet ready for review, open it as a **draft PR**. Convert
     it to ready for review only when it is complete and all CI checks pass.
   - Reference the issue your PR addresses in the description (e.g.,
     `Closes #123`).
   - Ensure the PR title follows the
     [Conventional Commits](#conventional-commits) format described below.

6. At least one Quent maintainer will be assigned to review the PR. Address any
   feedback and update the branch as needed.

## Conventional Commits

PR titles must follow the [Conventional Commits](https://www.conventionalcommits.org/)
specification. This is enforced by CI. The format is:

```text
<type>(<optional scope>): <subject>
```

Allowed types:

| Type       | When to use                                          |
|------------|------------------------------------------------------|
| `feat`     | A new feature                                        |
| `fix`      | A bug fix                                            |
| `docs`     | Documentation changes only                          |
| `style`    | Formatting, whitespace — no logic changes            |
| `refactor` | Code restructuring without behavior change           |
| `perf`     | Performance improvements                             |
| `test`     | Adding or updating tests                             |
| `build`    | Build system or dependency changes                   |
| `ci`       | CI configuration changes                             |
| `chore`    | Maintenance tasks that don't fit another type        |
| `revert`   | Reverts a previous commit                            |

Examples:

```text
feat(ui): add query timeline view
fix: correct FSM transition on empty result set
docs: add contributing guidelines
chore(deps): bump tokio to 1.48
```

Note: Individual commit messages within a PR do not need to follow this format —
only the PR title is validated. Commits do need a DCO sign-off line (see below).

## Use of AI Tools

AI-assisted development tools are permitted, but contributors are expected to
fully understand every change they submit. Reviewers may ask questions about any
part of your code during review — you should be able to explain the reasoning
behind your implementation, discuss trade-offs, and defend design decisions
without relying on the tool that generated the code.

PRs where it is apparent that the author does not understand the submitted code
will not be accepted.

## Coding Guidelines

- Keep pull requests focused. Each PR should address a single concern tied to
  its issue. If you find unrelated things to fix, open separate PRs.
- Avoid committing commented-out code.
- Do not introduce warnings — Rust CI runs `cargo clippy` with `-D warnings`.
- New components should include accompanying tests.

### Rust

Format and lint your changes before pushing:

```bash
# Format
pixi run cargo fmt --all

# Lint
pixi run cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

# Test
pixi run cargo test --workspace --all-features --locked --all-targets
```

### UI

Format, lint, and test your changes before pushing:

```bash
cd ui
pnpm ci:check
```

## CI Checks

All of the following checks must pass before a PR can be merged:

### PR title

- Conventional commit format

### Rust (runs when Rust or proto sources change)

- `cargo fmt` — formatting
- `cargo clippy` — lints, no warnings allowed
- `cargo test` — all tests pass
- `cargo build` — release build succeeds
- `cargo deny` — license and security audit

### UI (runs when `ui/` sources change)

- Format check
- ESLint
- TypeScript type check
- Tests with coverage
- `pnpm audit` — dependency security audit
- Production build

### Markdown (runs when Rust or proto sources change)

- `rumdl` — markdown lint

## Developer Certificate of Origin (DCO)

All contributions to this project must be made under the terms of the
[Developer Certificate of Origin (DCO)](https://developercertificate.org/), version 1.1.

By contributing to this project, you certify that:

1. The contribution was created in whole or in part by you and you have the
   right to submit it under the open source license indicated in the file; or
2. The contribution is based upon previous work that, to the best of your
   knowledge, is covered under an appropriate open source license and you have
   the right under that license to submit that work with modifications, whether
   created in whole or in part by you, under the same open source license
   (unless you are permitted to submit under a different license), as indicated
   in the file; or
3. The contribution was provided directly to you by some other person who
   certified (1), (2) or (3) and you have not modified it; or
4. You understand and agree that this project and the contribution are public
   and that a record of the contribution (including all personal information you
   submit with it, including your sign-off) is maintained indefinitely and may
   be redistributed consistent with this project or the open source license(s)
   involved.

To acknowledge that you agree to the DCO, sign off your commits by adding the
following line to your commit message (using your real name — no pseudonyms or
anonymous contributions):

```text
Signed-off-by: Jane Doe <jane.doe@example.com>
```

You can do this automatically by passing `-s` / `--signoff` to `git commit`:

```bash
git commit -s -m "Your commit message"
```

Any PR containing commits without a sign-off will not be accepted.

## License

By contributing to Quent, you agree that your contributions will be licensed
under the [Apache License, Version 2.0](LICENSE).

Each source file should include the following SPDX header:

```text
// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0
```
