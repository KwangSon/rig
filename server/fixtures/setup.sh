#!/bin/bash

# Fixture setup script for Rig
# This script migrates existing filesystem projects to the database

set -e

BASE_DIR="server/examples"
DB_URL="postgresql://kwang@localhost/rig"
ADMIN_USER_ID="550e8400-e29b-41d4-a716-446655440000"

echo "Migrating existing filesystem projects to database..."

# Find all project directories
find "$BASE_DIR" -mindepth 1 -maxdepth 1 -type d | while read -r project_dir; do
    project_name=$(basename "$project_dir")
    
    echo "Migrating project: $project_name"
    
    # Insert project into DB (ignore if exists)
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
    " 2>/dev/null || echo "Warning: Could not insert permission for $project_name"
    
done

echo "Database migration setup complete!"