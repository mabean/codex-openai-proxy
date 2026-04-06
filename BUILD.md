# BUILD

## Local build
```bash
cargo build --frozen
```

## Local tests
```bash
cargo test --frozen
```

## Optional lint
```bash
cargo clippy --all-targets --all-features -- -D warnings
```

## Current verification baseline
As of 2026-04-06, the repo has been checked locally with:
- `cargo fmt --check`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build`

## Notes
- `Cargo.lock` should be committed and kept current
- prefer reproducible builds (`--frozen`) in CI and local verification
- if vendoring is introduced later, document the exact vendor workflow here
