# Release Steps for mdviewer

## Prerequisites
- Branch is `fix/print-footer-robust` or feature branch ready to merge
- `make all` passes: `cargo test --lib`, `cargo clippy -- -D warnings`, `cargo fmt --check`
- Version bumped in both files:
  - `src-tauri/Cargo.toml` `version = "X.Y.Z"`
  - `src-tauri/tauri.conf.json` `"version": "X.Y.Z"`

## Steps
1. **Ensure build passes**
   ```bash
   make all
   ```

2. **Bump version**
   ```bash
   sed -i '' 's/version = ".*"/version = "1.8.5"/' src-tauri/Cargo.toml
   sed -i '' 's/"version": ".*"/"version": "1.8.5"/' src-tauri/tauri.conf.json
   git add -A && git commit -m "chore(release): bump version to 1.8.5"
   ```

3. **Create PR from feature branch to main**
   - Use `hub` or GitHub UI
   - PR title: `Release v1.8.5`
   - Body: summarize changes since last release

4. **Merge PR**
   - Merge to `main` via squash or merge commit
   - Ensure CI passes

5. **Tag release**
   ```bash
   git tag v1.8.5
   git push origin main --tags
   ```

6. **Watch GitHub Actions release workflow**
   - Workflow: `.github/workflows/release.yml`
   - Wait for build to complete and draft release to be created

7. **Update Release notes**
   - Open the draft release on GitHub
   - Add description of changes since last release
   - Mark as latest release if successful

8. **Update README Colophon & Agent Bookkeeping**
   - Update `README.md` Colophon `Commit attribution` section with current counts:
     - Count pi.dev commits, omp harness commits, Claude Code commits
     - Update model names/harnesses if changed
     - Keep attribution as of the new version tag
   - Commit README update separately:
     ```bash
     git add README.md
     git commit -m "docs: update README colophon with commit attribution for vX.Y.Z"
     git push origin main
     ```

## Notes
- Release workflow builds `.dmg` automatically
- Do not commit until tests pass
- Version must be bumped in both Cargo.toml and tauri.conf.json
