#!/bin/bash
set -e

# Setup paths
ROOT_DIR="$(pwd)"
RIG_BIN="$ROOT_DIR/target/debug/client"
PROJECT_NAME="branching_test_$(date +%s)"
SERVER_URL="http://localhost:3000"
API_URL="$SERVER_URL/api/v1"

ADMIN_EMAIL="admin@example.com"
ADMIN_PASSWORD="password"

function cleanup {
    echo -e "\n--- Cleaning up ---"
    if [ ! -z "$AUTH_TOKEN" ]; then
        curl -s -X DELETE "$API_URL/projects/$PROJECT_NAME" \
             -H "Authorization: Bearer $AUTH_TOKEN" > /dev/null
    fi
    cd "$ROOT_DIR"
    rm -rf "$PROJECT_NAME"
}
trap cleanup EXIT

echo "=== Starting Checkout & Stash (Shelving) Test ==="

# Login
LOGIN_RESP=$(curl -s -X POST "$API_URL/login" \
    -H "Content-Type: application/json" \
    -d "{\"email\":\"$ADMIN_EMAIL\", \"password\":\"$ADMIN_PASSWORD\"}")
AUTH_TOKEN=$(echo $LOGIN_RESP | sed -n 's/.*"token":"\([^"]*\)".*/\1/p')

# Create project
curl -s -X POST "$API_URL/create_project" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $AUTH_TOKEN" \
    -d "{\"name\":\"$PROJECT_NAME\"}" > /dev/null

# Clone
"$RIG_BIN" clone "ssh://rig@localhost:2222/$PROJECT_NAME" "$PROJECT_NAME" --username "DevUser"
cd "$PROJECT_NAME"

# Commit base file
echo "Base content" > base.txt
"$RIG_BIN" add base.txt
"$RIG_BIN" commit -m "Base commit"
"$RIG_BIN" push

# Create and checkout a branch
echo -e "\n--- 1. Switching to a new feature branch ---"
"$RIG_BIN" branch feat/advanced
"$RIG_BIN" checkout feat/advanced

# Modify a large file (simulate dirtiness)
echo -e "\n--- 2. Simulating unpushed work on feature branch ---"
echo "Huge binary data here..." > huge_asset.bin
"$RIG_BIN" add huge_asset.bin

dirty_checkout_output=$("$RIG_BIN" checkout main 2>&1 || true)
echo "$dirty_checkout_output"

if [[ "$dirty_checkout_output" == *"Checkout aborted"* ]]; then
    echo "PASS: Checkout correctly blocked due to dirty workspace!"
else
    echo "FAIL: Checkout allowed dirty workspace to switch!"
    exit 1
fi

# Shelve the changes
echo -e "\n--- 3. Running Remote Shelve (stash) ---"
"$RIG_BIN" stash

echo "Workspace should now be clean..."
clean_checkout_output=$("$RIG_BIN" checkout main 2>&1 || true)
echo "$clean_checkout_output"

if [[ "$clean_checkout_output" == *"Switched to branch"* ]]; then
    echo "PASS: Checkout succeeded after stash!"
else
    echo "FAIL: Checkout still failed after stash!"
    exit 1
fi

echo -e "\n=== Test Completed Successfully ==="
