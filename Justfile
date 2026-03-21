# SPDX-License-Identifier: PMPL-1.0-or-later
# typedqliser — Formal type safety (10 levels) for any query language

# Default: build and test
default: build test

# Build release binary
build:
    cargo build --release

# Run all tests
test:
    cargo test

# Run clippy lints
lint:
    cargo clippy -- -D warnings

# Format code
fmt:
    cargo fmt

# Check formatting without modifying
fmt-check:
    cargo fmt -- --check

# Build documentation
doc:
    cargo doc --no-deps --open

# Clean build artifacts
clean:
    cargo clean

# Run the CLI
run *ARGS:
    cargo run -- {{ARGS}}

# Full quality check (lint + test + fmt-check)
quality: fmt-check lint test
    @echo "All quality checks passed"

# Install locally
install:
    cargo install --path .

# Run panic-attacker pre-commit scan
assail:
    @command -v panic-attack >/dev/null 2>&1 && panic-attack assail . || echo "panic-attack not found — install from https://github.com/hyperpolymath/panic-attacker"

# --- Domain-Specific Recipes (typedqliser) ---

# Type-check a query at a given safety level\ncheck QUERY LEVEL='1':\n    cargo run -- check {{QUERY}} --level {{LEVEL}}\n\n# Validate query language grammar\nvalidate-grammar GRAMMAR:\n    cargo run -- validate-grammar {{GRAMMAR}}\n\n# Run type inference on a query file\ninfer QUERY_FILE:\n    cargo run -- infer {{QUERY_FILE}}

# Run contractile checks
contractile-check:
    @echo "Running contractile validation..."
    @test -f .machine_readable/contractiles/must/Mustfile.a2ml && echo "Mustfile: OK" || echo "Mustfile: MISSING"
    @test -f .machine_readable/contractiles/trust/Trustfile.a2ml && echo "Trustfile: OK" || echo "Trustfile: MISSING"
    @test -f .machine_readable/contractiles/dust/Dustfile.a2ml && echo "Dustfile: OK" || echo "Dustfile: MISSING"
    @test -f .machine_readable/contractiles/intend/Intendfile.a2ml && echo "Intendfile: OK" || echo "Intendfile: MISSING"

# RSR compliance check
rsr-check: quality contractile-check
    @echo "RSR compliance check complete"
