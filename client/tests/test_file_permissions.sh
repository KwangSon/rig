#!/bin/bash
set -e

# Setup paths
ROOT_DIR="$(pwd)"
RIG_BIN="$ROOT_DIR/target/debug/rig"
PROJECT_NAME="test_permissions_project_$(date +%s)"
CLONE_DIR_1="clone_user_1"
CLONE_DIR_2="clone_user_2"
SERVER_URL="http://localhost:3000"
API_URL="$SERVER_URL/api/v1"

# Credentials
ADMIN_EMAIL="admin@example.com"
ADMIN_PASSWORD="admin123"

# Save option check
SAVE_PROJECT=false
if [[ "$*" == *"--save"* ]]; then
    SAVE_PROJECT=true
fi

# Cleanup function
function cleanup {
    if [ "$SAVE_PROJECT" = true ]; then
        echo -e "\n--- Skipping cleanup (save mode) ---"
        return
    fi
    echo -e "\n--- Cleaning up ---"
    if [ ! -z "$AUTH_TOKEN" ]; then
        curl -s -X DELETE "$API_URL/projects/$PROJECT_NAME" -H "Authorization: Bearer $AUTH_TOKEN" > /dev/null
    fi
    cd "$ROOT_DIR"
    rm -rf "$PROJECT_NAME" "$CLONE_DIR_1" "$CLONE_DIR_2"
}
trap cleanup EXIT

echo "=== Starting File Permissions & Lock Test ==="

# 0. Login & Create Project
LOGIN_RESP=$(curl -s -X POST "$API_URL/login" -H "Content-Type: application/json" -d "{\"email\":\"$ADMIN_EMAIL\", \"password\":\"$ADMIN_PASSWORD\"}")
AUTH_TOKEN=$(echo $LOGIN_RESP | sed -n 's/.*"token":"\([^"]*\)".*/\1/p')

# Inject token for rig client
mkdir -p ~/.config/rig
echo "{\"host_tokens\":{\"$SERVER_URL\":\"$AUTH_TOKEN\"}}" > ~/.config/rig/credentials
chmod 600 ~/.config/rig/credentials

curl -s -X POST "$API_URL/create_project" -H "Content-Type: application/json" -H "Authorization: Bearer $AUTH_TOKEN" -d "{\"name\":\"$PROJECT_NAME\"}" > /dev/null

# 1. Clone & Add new local file
"$RIG_BIN" clone "$SERVER_URL/admin/$PROJECT_NAME" "$CLONE_DIR_1" --username "User1"
cd "$CLONE_DIR_1"

echo "New file content" > local_file.txt
echo "-> Testing 'add' for a new local file..."
"$RIG_BIN" add local_file.txt
"$RIG_BIN" commit -m "Add new local file"
"$RIG_BIN" push

# 2. Clone from another side and check read-only
cd "$ROOT_DIR"
"$RIG_BIN" clone "$SERVER_URL/admin/$PROJECT_NAME" "$CLONE_DIR_2" --username "User2"
cd "$CLONE_DIR_2"
"$RIG_BIN" pull local_file.txt

echo "-> Checking if pulled file is read-only..."
if [ -w "local_file.txt" ]; then
    echo "Error: Pulled file should be read-only!"
    exit 1
else
    echo "Success: Pulled file is read-only."
fi

# 3. Test lock to make it writable
echo "-> Testing 'lock' to make it writable..."
"$RIG_BIN" lock local_file.txt

if [ -w "local_file.txt" ]; then
    echo "Success: File is now writable after lock."
else
    echo "Error: File should be writable after lock!"
    exit 1
fi

echo "Modifying content" >> local_file.txt
"$RIG_BIN" add local_file.txt
"$RIG_BIN" commit -m "Modify after lock"
"$RIG_BIN" push

echo -e "\n=== File Permissions & Lock Test Completed Successfully! ==="
