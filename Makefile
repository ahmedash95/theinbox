.PHONY: dev build release install clean

# Development server
dev:
	pnpm tauri dev

# Build the app (debug)
build:
	pnpm tauri build --debug

# Build release version
release:
	pnpm tauri build

# Build and install to Applications folder
install: release
	@echo "Installing InboxCleanup to /Applications..."
	@rm -rf /Applications/InboxCleanup.app
	@cp -R src-tauri/target/release/bundle/macos/InboxCleanup.app /Applications/
	@echo "✅ InboxCleanup installed to /Applications"

# Quick install (skip build if already built)
install-quick:
	@if [ ! -d "src-tauri/target/release/bundle/macos/InboxCleanup.app" ]; then \
		echo "No release build found. Building..."; \
		pnpm tauri build; \
	fi
	@echo "Installing InboxCleanup to /Applications..."
	@rm -rf /Applications/InboxCleanup.app
	@cp -R src-tauri/target/release/bundle/macos/InboxCleanup.app /Applications/
	@echo "✅ InboxCleanup installed to /Applications"

# Clean build artifacts
clean:
	cd src-tauri && cargo clean
	rm -rf dist

# Show help
help:
	@echo "Available commands:"
	@echo "  make dev           - Start development server"
	@echo "  make build         - Build debug version"
	@echo "  make release       - Build release version"
	@echo "  make install       - Build release and install to /Applications"
	@echo "  make install-quick - Install to /Applications (skip build if exists)"
	@echo "  make clean         - Clean build artifacts"
