#!/bin/bash
set -e

# Setup paths
ROOT_DIR="$(pwd)"
RIG_BIN="$ROOT_DIR/target/debug/client"
PROJECT_NAME="full_workflow_test_$(date +%s)"
CLONE_DIR="full_workflow_cloned_project"
SERVER_URL="http://localhost:3000"
API_URL="$SERVER_URL/api/v1"

# Credentials (from setup.sh)
ADMIN_EMAIL="admin@example.com"
ADMIN_PASSWORD="password"

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
        echo "Local directories: $PROJECT_NAME, $CLONE_DIR"
        return
    fi

    echo -e "\n--- Cleaning up ---"
    if [ ! -z "$AUTH_TOKEN" ]; then
        echo "Deleting test project '$PROJECT_NAME' via API..."
        curl -s -X DELETE "$API_URL/projects/$PROJECT_NAME" \
             -H "Authorization: Bearer $AUTH_TOKEN" > /dev/null
    fi
    cd "$ROOT_DIR"
    rm -rf "$PROJECT_NAME" "$CLONE_DIR"
}
trap cleanup EXIT

# Ensure binary exists
if [ ! -f "$RIG_BIN" ]; then
    echo "Error: Rig binary not found at $RIG_BIN. Please run 'cargo build' first."
    exit 1
fi

echo "=== Starting Automated Full Workflow Test ==="

# 0. Login to get Token
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

# 1. Create project via API
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

# 2. Clone the new project
echo -e "\n--- 1. Cloning project: $PROJECT_NAME ---"
"$RIG_BIN" clone "$SERVER_URL/$PROJECT_NAME" "$PROJECT_NAME" --username "Jone"
cd "$PROJECT_NAME"

# 3. Add and Push first revision
echo -e "\n--- 2. Adding first artifact ---"
echo "Revision 1 content" > file1.txt
"$RIG_BIN" add file1.txt
"$RIG_BIN" commit -m "Initial commit with file1.txt"
"$RIG_BIN" push

# 4. Clone project into a new directory
cd "$ROOT_DIR"
echo -e "\n--- 3. Cloning project into $CLONE_DIR ---"
"$RIG_BIN" clone "$SERVER_URL/$PROJECT_NAME" "$CLONE_DIR" --username "CloneUser"

# 5. Pull and Modify from Clone
cd "$CLONE_DIR"
echo -e "\n--- 4. Pulling and modifying from clone ---"
"$RIG_BIN" pull file1.txt
echo "Original content in clone:"
cat file1.txt

"$RIG_BIN" lock file1.txt
echo "Revision 2 content from clone" >> file1.txt
"$RIG_BIN" commit -m "Revision 2 from cloned repository"
"$RIG_BIN" push

# 6. Verify History (Log)
echo -e "\n--- 5. Verifying history in clone ---"
"$RIG_BIN" log
echo ""
"$RIG_BIN" log file1.txt

# 7. Synchronize Original Repository
cd "$ROOT_DIR/$PROJECT_NAME"
echo -e "\n--- 6. Synchronizing original repository ---"
"$RIG_BIN" fetch
"$RIG_BIN" pull file1.txt
echo "Final content in original repo:"
cat file1.txt

echo -e "\n=== Full Workflow Test Completed Successfully! ==="
echo "You can now check the project '$PROJECT_NAME' in the Web UI."
