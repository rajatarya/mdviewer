# AGENTS.md — Markdown Viewer

## Project Goal

A **lightweight, fast** native Markdown viewer for macOS. Render GitHub-flavored and Obsidian-style Markdown (math, diagrams, callouts, emojis) as beautiful HTML in a native window with **instant load time** and a **small binary footprint**.

It is not a full-featured editor. It's a previewer — open a file, read it, close it. Simple, fast, reliable.

---

## Design Philosophy

### Priorities (in order)

1. **Binary size & load time** — Every dependency matters. Prefer zero or minimal dependencies. Avoid pulling in heavy crates for marginal gains.
2. **Simplicity** — Less code is better. A feature that adds 500 lines of Rust and 3 new deps is not worth it unless users actively ask for it.
3. **Reliability** — The renderer must never crash on malformed input. Sanitization is non-negotiable.
4. **Readability** — Code should be obvious to someone reading it for the first time. Name things clearly, keep functions small, avoid cleverness.

### What we do NOT want

- Feature bloat (no rich editing, no plugin system, no extensions)
- Heavy dependencies (avoid pulling in `tower`, `tokio` event loops, full web frameworks, etc.)
- Unnecessary abstraction layers — direct code is better than indirection
- Browser-based fallbacks or webviews with heavy JS bundles

---

## Code Standards

### General Principles

- **Minimal code**: Write the simplest solution that works. Resist the urge to add abstractions, patterns, or "just in case" scaffolding. If a feature can be done in 20 lines instead of 80, do it in 20.
- **Concise documentation**: Comments and docs should be brief and useful. Avoid restating what the code already says — explain the *why*. DESIGN.md entries should be one or two paragraphs at most.

### Rust (`src-tauri/src/`)

- **Formatting**: Run `cargo fmt` before every commit. No exceptions.
- **Linting**: Run `cargo clippy -- -D warnings`. Fix all warnings, do not suppress them unless there is a good reason (document why).
- **Naming**: Use snake_case for functions/variables, PascalCase for types/structs. Names should describe behavior, not implementation details.
- **Error handling**: Use `Result<T, String>` for user-facing errors where the message matters. Use `unwrap()` only in test code or when a failure is truly impossible.
- **Functions**: Keep functions under 50 lines. If longer, extract sub-functions with descriptive names. No function should require scrolling to understand its body.
- **Comments**: Document *why* not *what*. A comment explaining "we use a placeholder strategy because `regex` lacks lookbehind support" is valuable; one saying "// create a new Vec" is noise.

### Webview (`dist/index.html`)

- Keep it as a single file — no build step, no bundler. This is intentional and must stay this way.
- Vanilla JS only. No frameworks (React, Vue, Svelte, etc.). If you find yourself needing one, the code needs restructuring before adding dependencies.
- CSS uses custom properties (`--bg`, `--text`) for theming. Add new theme variables under `:root` and `[data-theme="dark"]`. Do not hardcode colors in component styles.
- Event handlers go at the end of the `<script>` block, grouped by feature.

### Tests

- Run `cargo test --lib` from the repo root before every commit. All tests must pass.
- Write tests for new features **before** implementation when practical (TDD). For UI changes, write Rust backend tests first, then verify manually in the app.
- Test names should describe the scenario: `test_read_file_success`, not `test1`.
- New test count = old test count + 1 per feature. Regression bugs add a new test that proves the fix works.

---

## Dependencies

### Rules

1. **Only use well-recognized and maintained dependencies.** Before adding any crate: check its GitHub stars, last release date, issue response time, and whether it has an active maintainer. Avoid obscure, unmaintained, or one-person projects with no recent updates.
2. **Check every new crate** before adding it — is there already one in our dependency tree? Can we do this with `std` or an existing crate?
3. **Lock to exact versions** in `Cargo.toml`. No `^` wildcards on major features. We control updates explicitly.
4. When a PR adds a new dependency, the commit message must say why and what problem it solves.

### Current dependencies (scan before adding)

- `pulldown-cmark` — markdown parsing ✓
- `ammonia` — HTML sanitization ✓
- `regex` — text preprocessing ✓
- `serde` / `serde_json` — data serialization ✓
- `tauri` — native app wrapper ✓
- `notify` (optional) — file watching, evaluate before adding

---

## Documentation

### DESIGN.md

The source of truth for architecture decisions and feature specifications. Update it when:
- A new feature is added
- An existing behavior changes
- A dependency version bump alters functionality

Keep descriptions brief but precise. Include code snippets where they clarify behavior.

### git commit messages

Use conventional-style format: `type(scope): description`

Types: `feat`, `fix`, `test`, `docs`, `chore`, `refactor`. Scope is optional but preferred when it clarifies what changed (e.g., `feat(core)`, `test(ui)`).

---

## Development Workflow

```bash
# From repo root — everything works from here:
cargo test        # Run all tests (must pass)
cargo build       # Build the app
cargo clippy      # Lint (no warnings allowed)
cargo fmt --check  # Format check (must be clean)
make all           # Run everything above in sequence

# To run the app:
make run           # Requires Tauri CLI installed (`npm install -g @tauri-apps/cli`)
```

### Before committing — checklist

- [ ] `cargo test --lib` passes (all tests green)
- [ ] `cargo clippy` has zero warnings
- [ ] `cargo fmt --check` is clean
- [ ] New features have new tests
- [ ] DESIGN.md updated if architecture changed
- [ ] Commit message follows the format

---

## What "Done" Looks Like

A feature or fix is done when:

1. Code compiles and all tests pass (`cargo test --lib`)
2. No clippy warnings remain
3. Formatting is applied (`cargo fmt`)
4. Changes are committed with a clear message
5. DESIGN.md reflects the change (if applicable)

If it meets these criteria, ship it. Don't over-engineer.
