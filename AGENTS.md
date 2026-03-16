# AGENTS.md

## Setup commands
- Install deps: `cargo add -p {target} xxx`

## Database
- Start PostgreSQL: `brew services start postgresql` (or your local setup)
- Setup database & fixtures: `./server/fixtures/setup.sh` (Resets DB, runs migrations, inserts test users and projects)
- Connect to DB: `psql postgresql://kwang@localhost/rig`

## API server
`DATABASE_URL=postgresql://kwang@localhost/rig cargo run -p server -- server/examples`

## Web Server
- Start web server: `cd server/web && bun dev`
- Access at http://localhost:3002

## Testing
- Start dev server: `cargo run -p server -- server/examples`
- Start client on new directory: `mkdir tmp, cd tmp, cargo run --manifest-path ../client/Cargo.toml`
- Script test: `client/tests/workflow.sh`

## After testing
- Format: `cargo fmt`
- Clippy: `cargo clippy`
