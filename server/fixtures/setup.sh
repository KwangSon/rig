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
psql "$DB_URL" -f "$MIGRATIONS_DIR/003_create_ssh_keys.sql"
psql "$DB_URL" -f "$MIGRATIONS_DIR/004_create_file_metadata.sql"

echo "Enabling pgcrypto extension for password hashing..."
psql "$DB_URL" -c "CREATE EXTENSION IF NOT EXISTS pgcrypto;"

# User IDs (fixed for fixtures)
ADMIN_USER_ID="550e8400-e29b-41d4-a716-446655440000"
REGULAR_USER_ID="550e8400-e29b-41d4-a716-446655440001"

echo "Setting up initial test users..."
echo "Admin: admin@example.com / $ADMIN_PASSWORD"
echo "User1: user1@example.com / $USER1_PASSWORD"

# Insert Users (Using pgcrypto to hash directly in SQL)
# Removed 'role' column as per Issue #5
psql "$DB_URL" -c "
INSERT INTO users (id, name, email, password_hash) VALUES
('$ADMIN_USER_ID'::uuid, 'Admin', 'admin@example.com', crypt('$ADMIN_PASSWORD', gen_salt('bf'))),
('$REGULAR_USER_ID'::uuid, 'User1', 'user1@example.com', crypt('$USER1_PASSWORD', gen_salt('bf')))
ON CONFLICT (email) DO NOTHING;
" 2>/dev/null || echo "Warning: Could not insert test users"

echo "Migrating existing filesystem projects to database..."

# Find all project directories
find "$BASE_DIR" -mindepth 1 -maxdepth 1 -type d | while read -r project_dir; do
    project_name=$(basename "$project_dir")
    
    echo "Migrating project: $project_name"
    
    # Insert project into DB and get the ID
    project_id=$(psql "$DB_URL" -t -A -c "
    INSERT INTO projects (name, owner_id) 
    VALUES ('$project_name', '$ADMIN_USER_ID'::uuid)
    ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name
    RETURNING id;
    ")
    
    if [ -z "$project_id" ]; then
        echo "Warning: Could not get ID for project $project_name"
        continue
    fi
    
    # Insert admin permission for owner (using project_id UUID)
    psql "$DB_URL" -c "
    INSERT INTO permissions (user_id, project_id, access) 
    VALUES ('$ADMIN_USER_ID'::uuid, '$project_id'::uuid, 'admin')
    ON CONFLICT (user_id, project_id) DO NOTHING;
    " 2>/dev/null || echo "Warning: Could not insert admin permission for $project_name"

    # Also insert read permission for the regular user to ExampleProject specifically
    if [ "$project_name" == "ExampleProject" ]; then
        psql "$DB_URL" -c "
        INSERT INTO permissions (user_id, project_id, access) 
        VALUES ('$REGULAR_USER_ID'::uuid, '$project_id'::uuid, 'read')
        ON CONFLICT (user_id, project_id) DO NOTHING;
        " 2>/dev/null || echo "Warning: Could not insert read permission for User1 on ExampleProject"
    fi
    
done

echo "Database setup and migration complete!"
