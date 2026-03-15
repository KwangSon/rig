# AGENTS.md

## Setup commands
- Install deps: `cargo add -p {target} xxx`

## Testing
- Start dev server: `cargo run -p server -- server/examples`
- Start client on new directory: `mkdir tmp, cd tmp, cargo run --manifest-path ../client/Cargo.toml`

## After testing
- Format: `cargo fmt`
- Clippy: `cargo clippy`
