# AGENTS.md

## Setup commands
- Install deps: `cargo add -p {target} xxx`

## Database
- Start PostgreSQL: `brew services start postgresql` (or your local setup)
- Connect to DB: `psql postgresql://kwang@localhost/rig`
- For schema changes (initial phase): Drop DB, modify SQL files, recreate DB
  - Drop: `dropdb rig` (run in shell, not inside psql)
  - Recreate: `createdb rig` (run in shell, not inside psql)
  - Run schema: `psql postgresql://kwang@localhost/rig < server/migrations/001_create_users.sql` and `psql postgresql://kwang@localhost/rig < server/migrations/002_create_permissions.sql` (includes fixture data)
- Run fixture setup: `./server/fixtures/setup.sh` (migrates all directories in server/examples to DB as projects)

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
