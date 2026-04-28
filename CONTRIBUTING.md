# Contributing to Markdown Viewer

Thanks for your interest in this project!

## We welcome:

### 🐛 Bug reports and feature requests

Open an [issue](https://github.com/rajatarya/mdviewer/issues) — that's the easiest way to contribute. Clear reproduction steps, screenshots, and a description of what you expected go a long way.

### 🍴 Forks and personal tweaks

Feel free to fork this repo and customize it for your own workflow. That's what open source is for. If your fork introduces something interesting, a PR is welcome — but a fork is just as valuable.

### ✅ Pull requests

We accept PRs for:

- Bug fixes (with a test that proves it)
- Small, well-scoped improvements
- Documentation fixes
- Performance optimizations with measured before/after

## We'd prefer you avoid:

### 🤖 Blind AI-generated code

This project was itself written by an AI agent — and we learned that **AI-generated code without deep review is a liability**. If you use AI to draft code:

- **Understand every line** before submitting
- **Run the tests** and verify locally
- **Explain your changes** in the PR description — if you can't, you shouldn't submit it
- Don't submit mass-generated diffs with vague descriptions

We'll merge PRs that show genuine understanding. We'll close PRs that look like a blind `Ctrl+Enter` away from a chat window.

### 📦 Dependency bloat

Every new crate has a long-term cost. Before proposing a dependency:

- Can this be done with existing deps or `std`?
- Is it actively maintained?
- What's the compile-time and binary-size impact?

If the answer isn't clear, don't add it.

### 🎨 Massive refactors

We value simple, readable code over clever abstractions. A PR that touches 500+ lines to "improve architecture" will be declined. Small, targeted changes win.

## PR checklist

- [ ] `cargo test --lib` passes
- [ ] `cargo clippy` has zero warnings
- [ ] `cargo fmt` is clean
- [ ] Changes are scoped to a single concern
- [ ] Commit message follows the conventional format

## License

By contributing, you agree that your contributions are licensed under the [MIT License](LICENSE).
