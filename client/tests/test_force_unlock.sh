#!/bin/bash
set -e

# Setup paths
ROOT_DIR="$(pwd)"
RIG_BIN="$ROOT_DIR/target/debug/client"
PROJECT_NAME="test_force_unlock_project_$(date +%s)"
CLONE_DIR_1="clone_user_1"
CLONE_DIR_2="clone_admin"
SERVER_URL="http://localhost:3000"
API_URL="$SERVER_URL/api/v1"

# Credentials (from setup.sh)
ADMIN_EMAIL="admin@example.com"
ADMIN_PASSWORD=""
USER1_EMAIL="user1@example.com"
USER1_PASSWORD="password"

# Save option check
SAVE_PROJECT=false
if [[ "$*" == *"--save"* ]]; then
    SAVE_PROJECT=true
    echo "-> Save mode enabled: Project and local files will be preserved."
fi

# Cleanup function
function cleanup {
    if [ "$SAVE_PROJECT" = true ]; then
        echo -e "\n--- Skipping cleanup as requested (save mode) ---"
        echo "Project: $PROJECT_NAME"
        echo "Local directories: $CLONE_DIR_1, $CLONE_DIR_2"
        return
    fi

    echo -e "\n--- Cleaning up test directories ---"
    if [ ! -z "$AUTH_TOKEN" ]; then
        echo "Deleting test project '$PROJECT_NAME' via API..."
        curl -s -X DELETE "$API_URL/projects/$PROJECT_NAME" \
             -H "Authorization: Bearer $AUTH_TOKEN" > /dev/null
    fi
    cd "$ROOT_DIR"
    rm -rf "$PROJECT_NAME" "$CLONE_DIR_1" "$CLONE_DIR_2"
}
trap cleanup EXIT

# Ensure binary exists
if [ ! -f "$RIG_BIN" ]; then
    echo "Error: Rig binary not found at $RIG_BIN. Please run 'cargo build' first."
    exit 1
fi

echo "=== Starting Force Unlock Test ==="

# 0. Login to get Token for Admin
echo "-> Logging in as admin..."
LOGIN_RESP=$(curl -s -X POST "$API_URL/login" \
    -H "Content-Type: application/json" \
    -d "{\"email\":\"$ADMIN_EMAIL\", \"password\":\"$ADMIN_PASSWORD\"}")

AUTH_TOKEN=$(echo $LOGIN_RESP | sed -n 's/.*"token":"\([^"]*\)".*/\1/p')

if [ -z "$AUTH_TOKEN" ]; then
    echo "Error: Failed to login and get auth token."
    echo "Response: $LOGIN_RESP"
    exit 1
fi

# 1. Create project via API (Admin is the owner)
echo "-> Creating project '$PROJECT_NAME' via API..."
CREATE_RESP=$(curl -s -X POST "$API_URL/create_project" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $AUTH_TOKEN" \
    -d "{\"name\":\"$PROJECT_NAME\"}")

if [[ $CREATE_RESP == *"error"* ]]; then
    echo "Error: Failed to create project."
    echo "Response: $CREATE_RESP"
    exit 1
fi

# 2. Clone for User 1 (Regular User) and Admin
cd "$ROOT_DIR"
echo -e "\n--- 2. Cloning for User 1 and Admin ---"
"$RIG_BIN" clone "$SERVER_URL/admin/$PROJECT_NAME" "$CLONE_DIR_1" --username "User1"
"$RIG_BIN" clone "$SERVER_URL/admin/$PROJECT_NAME" "$CLONE_DIR_2" --username "Admin"

# 3. Add initial file as Admin and Push
cd "$ROOT_DIR/$CLONE_DIR_2"
echo "Initial content" > file.txt
"$RIG_BIN" add file.txt
"$RIG_BIN" commit -m "Initial commit"
"$RIG_BIN" push

# 4. User 1 pulls and locks the file
cd "$ROOT_DIR/$CLONE_DIR_1"
echo -e "\n--- 3. User 1 pulling and locking file ---"
"$RIG_BIN" pull file.txt
"$RIG_BIN" lock file.txt

# 5. Admin tries to unlock it without force (should fail because it's locked by User1)
cd "$ROOT_DIR/$CLONE_DIR_2"
echo -e "\n--- 4. Admin trying to unlock User 1's file (should fail without --force) ---"
if "$RIG_BIN" unlock file.txt; then
    echo "Error: Admin should NOT be able to unlock User 1's file without --force"
    exit 1
else
    echo "Success: Admin failed to unlock without --force (as expected)"
fi

# 6. Admin force unlocks it (should succeed)
echo -e "\n--- 5. Admin force unlocking file (should succeed) ---"
"$RIG_BIN" unlock file.txt --force
echo "Success: Admin force unlocked the file"

# 7. Admin can now lock it
echo -e "\n--- 6. Admin can now lock the file ---"
"$RIG_BIN" lock file.txt
echo "Success: Admin locked the file after force unlock"

echo -e "\n=== Force Unlock Test Completed Successfully! ==="
