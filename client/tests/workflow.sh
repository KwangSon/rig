#!/bin/bash

# Multi-User Collaboration Test for Rig (HTTP Token Auth)
# This script simulates two users working on the same project.

set -e

# Configuration
SERVER_URL="http://localhost:3000"
API_URL="$SERVER_URL/api/v1"
PROJECT_NAME="WorkflowTest_$(date +%s)"
ROOT_DIR="$(pwd)"
RIG_BIN="$ROOT_DIR/target/debug/rig"
ADMIN_WS="$ROOT_DIR/admin_ws"
USER1_WS="$ROOT_DIR/user1_ws"

# Tokens (from setup.sh)
ADMIN_TOKEN="rigp_admin1234567890"
USER1_TOKEN="rigp_user11234567890"

# Cleanup function
function cleanup {
    echo -e "\n--- Cleaning up test workspaces ---"
    rm -rf "$ADMIN_WS" "$USER1_WS"
    rm -f ~/.config/rig/credentials
}
trap cleanup EXIT

# Ensure binary exists
if [ ! -f "$RIG_BIN" ]; then
    echo "Error: Rig binary not found at $RIG_BIN. Please run 'cargo build' first."
    exit 1
fi

echo "=== Starting Multi-User Collaboration Test ==="

# Helper to setup credentials for a specific user
function set_token {
    local token=$1
    mkdir -p ~/.config/rig
    echo "{\"host_tokens\":{\"$SERVER_URL\":\"$token\"}}" > ~/.config/rig/credentials
    chmod 600 ~/.config/rig/credentials
}

# --- 0. Create project via API ---
echo "--- 0. Creating project '$PROJECT_NAME' ---"
CREATE_RESP=$(curl -s -X POST "$API_URL/create_project" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d "{\"name\":\"$PROJECT_NAME\"}")
echo "Response: $CREATE_RESP"

# --- 1. User: Admin (Setting up project) ---
echo -e "\n--- 1. User: Admin (Setting up project) ---"
set_token "$ADMIN_TOKEN"
rm -rf "$ADMIN_WS"
"$RIG_BIN" clone "$SERVER_URL/admin/$PROJECT_NAME" "$ADMIN_WS" --username "Admin"

cd "$ADMIN_WS"
echo "Hello from Admin at $(date)" > shared_file.txt
"$RIG_BIN" add shared_file.txt
"$RIG_BIN" commit -m "Admin: Initial shared file"
"$RIG_BIN" push
cd "$ROOT_DIR"

# --- 2. User: User1 (Modifying file) ---
echo -e "\n--- 2. User: User1 (Modifying file) ---"
set_token "$USER1_TOKEN"
rm -rf "$USER1_WS"
"$RIG_BIN" clone "$SERVER_URL/admin/$PROJECT_NAME" "$USER1_WS" --username "User1"

cd "$USER1_WS"
"$RIG_BIN" pull shared_file.txt
"$RIG_BIN" lock shared_file.txt
echo "User1 added this line" >> shared_file.txt
"$RIG_BIN" add shared_file.txt
"$RIG_BIN" commit -m "User1: Updated shared file"

# --- 3. Admin conflict test (While User1 still holds the lock) ---
echo -e "\n--- 3. User: Admin (Testing lock conflict) ---"
set_token "$ADMIN_TOKEN"
cd "$ADMIN_WS"
"$RIG_BIN" fetch
echo "-> Attempting to lock file held by User1 (should fail)..."
# Use a subshell to avoid exiting on expected failure
if "$RIG_BIN" lock shared_file.txt 2>&1 | grep -q "locked by"; then
    echo "   Correct: Lock rejected because User1 holds it."
else
    echo "   Error: Lock should have been rejected!"
    exit 1
fi
cd "$ROOT_DIR"

# --- 4. User1 pushes and releases lock ---
echo -e "\n--- 4. User: User1 (Pushing and releasing lock) ---"
set_token "$USER1_TOKEN"
cd "$USER1_WS"
"$RIG_BIN" push
"$RIG_BIN" unlock shared_file.txt
cd "$ROOT_DIR"

# --- 5. User: Admin (Finalizing) ---
echo -e "\n--- 5. User: Admin (Finalizing) ---"
set_token "$ADMIN_TOKEN"
cd "$ADMIN_WS"
"$RIG_BIN" pull shared_file.txt
"$RIG_BIN" lock shared_file.txt
echo "Final file content in Admin workspace:"
cat shared_file.txt

echo -e "\n=== Test Passed Successfully! ==="
