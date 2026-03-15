# AGENTS.md

## Setup commands
- Install deps: `cargo add -p {target} xxx`

## Database
- Start PostgreSQL: `brew services start postgresql` (or your local setup)
- Connect to DB: `psql postgresql://kwang@localhost/rig`
- For schema changes (initial phase): Drop DB, modify SQL files, recreate DB
  - Drop: `dropdb rig`
  - Recreate: `createdb rig`
  - Run schema: `psql postgresql://kwang@localhost/rig < server/migrations/001_create_users.sql` etc.

## Web Server
- Start web server: `cd server/web && npm run dev` or `bun dev`
- Access at http://localhost:3000

## Testing
- Start dev server: `cargo run -p server -- server/examples`
- Start client on new directory: `mkdir tmp, cd tmp, cargo run --manifest-path ../client/Cargo.toml`
- Script test: `client/tests/workflow.sh`

## After testing
- Format: `cargo fmt`
- Clippy: `cargo clippy`
