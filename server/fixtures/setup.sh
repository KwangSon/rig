#!/bin/bash

# Rig Setup & Fixture Script
# This script resets the database, runs migrations, and sets up initial data.

set -e

DB_NAME="rig"
DB_URL="postgresql://kwang@localhost/$DB_NAME"
BASE_DIR="server/examples"

# --- [EDIT HERE] ---
# You can set plain text passwords here!
ADMIN_PASSWORD="password"
USER1_PASSWORD="password"
# ------------------

echo "Resetting database '$DB_NAME'..."
dropdb --if-exists "$DB_NAME"
createdb "$DB_NAME"

echo "Running migrations..."
psql "$DB_URL" -f server/migrations/001_create_users.sql
psql "$DB_URL" -f server/migrations/002_create_permissions.sql

echo "Enabling pgcrypto extension for password hashing..."
psql "$DB_URL" -c "CREATE EXTENSION IF NOT EXISTS pgcrypto;"

# User IDs (fixed for fixtures)
ADMIN_USER_ID="550e8400-e29b-41d4-a716-446655440000"
REGULAR_USER_ID="550e8400-e29b-41d4-a716-446655440001"

echo "Setting up initial test users..."
echo "Admin: admin@example.com / $ADMIN_PASSWORD"
echo "User1: user1@example.com / $USER1_PASSWORD"

# Insert Users (Using pgcrypto to hash directly in SQL)
psql "$DB_URL" -c "
INSERT INTO users (id, name, email, password_hash, role) VALUES
('$ADMIN_USER_ID'::uuid, 'Admin', 'admin@example.com', crypt('$ADMIN_PASSWORD', gen_salt('bf')), 'admin'),
('$REGULAR_USER_ID'::uuid, 'User1', 'user1@example.com', crypt('$USER1_PASSWORD', gen_salt('bf')), 'user')
ON CONFLICT (email) DO NOTHING;
" 2>/dev/null || echo "Warning: Could not insert test users"

echo "Migrating existing filesystem projects to database..."

# Find all project directories
find "$BASE_DIR" -mindepth 1 -maxdepth 1 -type d | while read -r project_dir; do
    project_name=$(basename "$project_dir")
    
    echo "Migrating project: $project_name"
    
    # Insert project into DB
    psql "$DB_URL" -c "
    INSERT INTO projects (name, owner_id) 
    VALUES ('$project_name', '$ADMIN_USER_ID'::uuid)
    ON CONFLICT (name) DO NOTHING;
    " 2>/dev/null || echo "Warning: Could not insert project $project_name"
    
    # Insert admin permission for owner
    psql "$DB_URL" -c "
    INSERT INTO permissions (user_id, project, access) 
    VALUES ('$ADMIN_USER_ID'::uuid, '$project_name', 'admin')
    ON CONFLICT (user_id, project) DO NOTHING;
    " 2>/dev/null || echo "Warning: Could not insert admin permission for $project_name"

    # Also insert read permission for the regular user to ExampleProject specifically
    if [ "$project_name" == "ExampleProject" ]; then
        psql "$DB_URL" -c "
        INSERT INTO permissions (user_id, project, access) 
        VALUES ('$REGULAR_USER_ID'::uuid, 'ExampleProject', 'read')
        ON CONFLICT (user_id, project) DO NOTHING;
        " 2>/dev/null || echo "Warning: Could not insert read permission for User1 on ExampleProject"
    fi
    
done

echo "Database setup and migration complete!"
