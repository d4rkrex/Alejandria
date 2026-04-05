.PHONY: help coverage coverage-no-open coverage-clean

# Default target: display help
help:
	@echo "Available make targets:"
	@echo "  help             - Display this help message"
	@echo "  coverage         - Generate HTML coverage report and open in browser"
	@echo "  coverage-no-open - Generate HTML coverage report without opening browser"
	@echo "  coverage-clean   - Remove all generated coverage artifacts"

# Generate and open HTML coverage report
coverage:
	@command -v cargo-tarpaulin >/dev/null 2>&1 || { \
		echo "Error: cargo-tarpaulin not found"; \
		echo "Install with: cargo install cargo-tarpaulin"; \
		exit 127; \
	}
	@echo "Generating coverage report..."
	@cargo tarpaulin --all-features --out Html --output-dir target/coverage --timeout 600
	@echo "Opening coverage report in browser..."
	@if command -v xdg-open >/dev/null 2>&1; then \
		xdg-open target/coverage/html/index.html; \
	elif command -v open >/dev/null 2>&1; then \
		open target/coverage/html/index.html; \
	else \
		echo "Browser could not be opened automatically."; \
		echo "View the report at: file://$(shell pwd)/target/coverage/html/index.html"; \
	fi

# Generate coverage report without opening browser
coverage-no-open:
	@command -v cargo-tarpaulin >/dev/null 2>&1 || { \
		echo "Error: cargo-tarpaulin not found"; \
		echo "Install with: cargo install cargo-tarpaulin"; \
		exit 127; \
	}
	@echo "Generating coverage report..."
	@cargo tarpaulin --all-features --out Html --output-dir target/coverage --timeout 600
	@echo "Coverage report generated at: file://$(shell pwd)/target/coverage/html/index.html"

# Remove all generated coverage artifacts
coverage-clean:
	@echo "Cleaning coverage reports..."
	@rm -rf target/coverage/
	@echo "Coverage reports cleaned"
