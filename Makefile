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

# Install the bundled app to ~/Applications and create CLI symlink
install: bundle
	./bin/install.sh

# Install without rebuilding (use after a fresh bundle)
install-fast:
	./bin/install.sh --no-build

# Uninstall: remove symlink and file associations
uninstall:
	./bin/install.sh --uninstall

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
