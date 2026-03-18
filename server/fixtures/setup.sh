#!/bin/bash

# Rig Setup & Fixture Script
# This script resets the database, runs migrations, and sets up initial data.

set -e

DB_NAME="rig"
DB_URL="postgresql://kwang@localhost/$DB_NAME"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RIG_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
MIGRATIONS_DIR="$RIG_ROOT/server/migrations"
BASE_DIR="$RIG_ROOT/server/examples"

echo "Resetting database '$DB_NAME'..."
dropdb --if-exists "$DB_NAME"
createdb "$DB_NAME"

echo "Running migrations..."
psql "$DB_URL" -f "$MIGRATIONS_DIR/001_create_users.sql"
psql "$DB_URL" -f "$MIGRATIONS_DIR/002_create_permissions.sql"
psql "$DB_URL" -f "$MIGRATIONS_DIR/004_create_file_metadata.sql"
psql "$DB_URL" -f "$MIGRATIONS_DIR/005_create_auth_tokens.sql"

echo "Enabling pgcrypto extension for password hashing..."
psql "$DB_URL" -c "CREATE EXTENSION IF NOT EXISTS pgcrypto;"

# User IDs (fixed for fixtures)
ADMIN_USER_ID="550e8400-e29b-41d4-a716-446655440000"
REGULAR_USER_ID="550e8400-e29b-41d4-a716-446655440001"

echo "Setting up initial test users..."
# Fixed passwords for tokens (normally hashed)
psql "$DB_URL" -c "
INSERT INTO users (id, name, email, password_hash) VALUES
('$ADMIN_USER_ID'::uuid, 'Admin', 'admin@example.com', crypt('admin123', gen_salt('bf'))),
('$REGULAR_USER_ID'::uuid, 'User1', 'user1@example.com', crypt('user1123', gen_salt('bf')))
ON CONFLICT (email) DO NOTHING;
" 

echo "Setting up predefined API tokens for testing..."
# Admin Token: rigp_admin1234567890
# User1 Token: rigp_user11234567890
psql "$DB_URL" -c "
INSERT INTO tokens (user_id, token_text, name) VALUES
('$ADMIN_USER_ID'::uuid, 'rigp_admin1234567890', 'test-admin-token'),
('$REGULAR_USER_ID'::uuid, 'rigp_user11234567890', 'test-user1-token')
ON CONFLICT (token_text) DO NOTHING;
"

echo "Migrating existing filesystem projects to database..."

# Find all project directories
find "$BASE_DIR" -mindepth 1 -maxdepth 1 -type d | while read -r project_dir; do
    project_name=""
    if [ -f "$project_dir/index" ]; then
        project_name=$(grep -Eo '"project": *"[^"]+"' "$project_dir/index" | head -1 | awk -F '"' '{print $4}')
    fi
    if [ -z "$project_name" ]; then
        project_name=$(basename "$project_dir")
    fi
    
    echo "Migrating project: $project_name (from $project_dir)"
    
    # Insert project into DB and get the ID
    project_id=$(psql "$DB_URL" -t -A -c "
    INSERT INTO projects (name, owner_id) 
    VALUES ('$project_name', '$ADMIN_USER_ID'::uuid)
    ON CONFLICT (owner_id, name) DO UPDATE SET name = EXCLUDED.name
    RETURNING id;
    " | head -n 1)
    
    if [ -z "$project_id" ]; then
        echo "Warning: Could not get ID for project $project_name"
        continue
    fi
    
    # Rename the directory to the generated UUID
    if [ "$project_name" != "$project_id" ]; then
        new_dir="$BASE_DIR/$project_id"
        echo "-> Renaming directory $project_name to $project_id"
        mv "$project_dir" "$new_dir"
    fi
    
    # Insert admin permission for owner (using project_id UUID)
    psql "$DB_URL" -c "
    INSERT INTO permissions (user_id, project_id, access) 
    VALUES ('$ADMIN_USER_ID'::uuid, '$project_id'::uuid, 'admin')
    ON CONFLICT (user_id, project_id) DO NOTHING;
    " || echo "Warning: Could not insert admin permission for $project_name"

    # Also insert write permission for the regular user to ExampleProject specifically
    if [ "$project_name" == "ExampleProject" ]; then
        psql "$DB_URL" -c "
        INSERT INTO permissions (user_id, project_id, access) 
        VALUES ('$REGULAR_USER_ID'::uuid, '$project_id'::uuid, 'write')
        ON CONFLICT (user_id, project_id) DO NOTHING;
        " || echo "Warning: Could not insert write permission for User1 on ExampleProject"
    fi
    
done

echo "Database setup and migration complete!"
