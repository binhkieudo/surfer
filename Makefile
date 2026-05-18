.PHONY: help setup debug release clean install uninstall test check fmt lint

# Default target
help:
	@echo "Surfer Build Targets"
	@echo "===================="
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@echo "  debug           Build debug version (unoptimized, fast compile)"
	@echo "  release         Build release version (optimized, slow compile)"
	@echo "  clean           Remove all build artifacts"
	@echo "  install         Install surfer to ~/.cargo/bin/"
	@echo "  uninstall       Uninstall surfer"
	@echo ""
	@echo "Development:"
	@echo "  test            Run all tests"
	@echo "  check           Check code without building"
	@echo "  fmt             Format code (cargo fmt)"
	@echo "  lint            Run clippy (linter)"
	@echo ""
	@echo "Advanced:"
	@echo "  doc             Build documentation"
	@echo "  bench           Run benchmarks"
	@echo ""
	@echo "Setup:"
	@echo "  setup           Initialize git submodules (required first time)"
	@echo ""

# Setup
setup:
	@echo "Initializing git submodules..."
	git submodule update --init --recursive
	@echo "✓ Submodules initialized"

# Build targets
debug: setup
	@echo "Building Surfer (debug)..."
	cargo build
	@echo "✓ Debug build complete: target/debug/surfer"

release: setup
	@echo "Building Surfer (release)..."
	cargo build --release
	@echo "✓ Release build complete: target/release/surfer"

# Cleanup
clean:
	@echo "Cleaning build artifacts..."
	cargo clean
	@echo "✓ Clean complete"

# Installation
install: release
	@echo "Installing Surfer..."
	cargo install --path surfer
	@echo "✓ Installed to ~/.cargo/bin/surfer"

uninstall:
	@echo "Uninstalling Surfer..."
	cargo uninstall surfer
	@echo "✓ Uninstalled"

# Development targets
test:
	@echo "Running tests..."
	cargo test --lib

check:
	@echo "Checking code..."
	cargo check

fmt:
	@echo "Formatting code..."
	cargo fmt

lint:
	@echo "Running clippy (linter)..."
	cargo clippy --all-targets --all-features -- -D warnings

# Documentation
doc:
	@echo "Building documentation..."
	cargo doc --no-deps --open

# Benchmarks
bench:
	@echo "Running benchmarks..."
	cargo bench

# Additional useful targets
run-debug: debug
	@echo "Running debug build..."
	./target/debug/surfer

run-release: release
	@echo "Running release build..."
	./target/release/surfer

size-debug: debug
	@echo "Debug binary size:"
	@ls -lh target/debug/surfer

size-release: release
	@echo "Release binary size:"
	@ls -lh target/release/surfer

# Print info
info:
	@echo "Surfer Project Info"
	@echo "==================="
	@echo "Repository: https://gitlab.com/surfer-project/surfer"
	@echo "Rust version required: 1.92+"
	@echo ""
	@echo "Build times (approx):"
	@echo "  Debug:   8-15 seconds (incremental)"
	@echo "  Release: 15-30 seconds (first build)"
	@echo "  Release: 50+ seconds (full rebuild)"
	@echo ""
	@echo "Binary sizes (approx):"
	@echo "  Debug:   800 MB"
	@echo "  Release: 400 MB"
	@echo ""
	@echo "Current branch:"
	@git branch --show-current
	@echo "Latest commit:"
	@git log -1 --oneline

# CI-like target (run checks before commit)
pre-commit: check lint fmt
	@echo "✓ Pre-commit checks passed"

# Full clean and rebuild
rebuild: clean debug
	@echo "✓ Full rebuild complete"

rebuild-release: clean release
	@echo "✓ Full release rebuild complete"
