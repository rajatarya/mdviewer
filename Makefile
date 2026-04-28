.PHONY: test build run fmt clippy clean all check

# Run all tests (Rust)
test:
	cd src-tauri && cargo test --lib

# Run all tests (Rust) with output
test-verbose:
	cd src-tauri && cargo test --lib -- --nocapture

# Build the Tauri app
build:
	cd src-tauri && cargo build

# Build in release mode (binary only)
release:
	cd src-tauri && cargo build --release

# Build and bundle the macOS app (creates .app and .dmg)
bundle:
	cd src-tauri && cargo tauri build

# Install the bundled app to ~/Applications
install: bundle
	cp -R target/release/bundle/macos/*.app ~/Applications/
	xattr -dr com.apple.quarantine ~/Applications/*.app 2>/dev/null || true
	@echo "Installed to ~/Applications/"

# Set as default app for .md files (requires: brew install duti)
default: install
	duti -s app.mdviewer .md all
	@echo "Markdown Viewer is now the default for .md files"

# Run the Tauri app (requires Tauri CLI)
run:
	cd src-tauri && cargo tauri dev

# Format all code
fmt:
	cd src-tauri && cargo fmt

# Check for warnings and errors (no build)
check:
	cd src-tauri && cargo check

# Run clippy
clippy:
	cd src-tauri && cargo clippy -- -D warnings

# Clean build artifacts
clean:
	cd src-tauri && cargo clean

# Run everything: fmt, clippy, test, build
all: fmt clippy test build
