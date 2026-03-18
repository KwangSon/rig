#!/bin/bash
set -e

# Setup paths
ROOT_DIR="$(pwd)"
RIG_BIN="$ROOT_DIR/target/debug/client"
PROJECT_NAME="test_pull_rev_$(date +%s)"
CLONE_DIR="clone_test_rev"
SERVER_URL="http://localhost:3000"
API_URL="$SERVER_URL/api/v1"

# Credentials
ADMIN_EMAIL="admin@example.com"
ADMIN_PASSWORD="admin123"

# Cleanup
function cleanup {
    echo -e "\n--- Cleaning up ---"
    cd "$ROOT_DIR"
    rm -rf "$PROJECT_NAME" "$CLONE_DIR"
}
trap cleanup EXIT

echo "=== Starting Pull Revision/Commit Test ==="

# 0. Login & Create Project
LOGIN_RESP=$(curl -s -X POST "$API_URL/login" -H "Content-Type: application/json" -d "{\"email\":\"$ADMIN_EMAIL\", \"password\":\"$ADMIN_PASSWORD\"}")
AUTH_TOKEN=$(echo $LOGIN_RESP | sed -n 's/.*"token":"\([^"]*\)".*/\1/p')

# Inject token for rig client
mkdir -p ~/.config/rig
echo "{\"host_tokens\":{\"$SERVER_URL\":\"$AUTH_TOKEN\"}}" > ~/.config/rig/credentials
chmod 600 ~/.config/rig/credentials

curl -s -X POST "$API_URL/create_project" -H "Content-Type: application/json" -H "Authorization: Bearer $AUTH_TOKEN" -d "{\"name\":\"$PROJECT_NAME\"}" > /dev/null

# 1. Clone & Push 2 revisions
"$RIG_BIN" clone "$SERVER_URL/admin/$PROJECT_NAME" "$PROJECT_NAME" --username "User1"
cd "$PROJECT_NAME"

echo "Content Rev 1" > file.txt
"$RIG_BIN" add file.txt
"$RIG_BIN" commit -m "Rev 1"
"$RIG_BIN" push
# Log format: <hash> <message> - <author>
COMMIT1_HASH=$("$RIG_BIN" log | grep "Rev 1" | awk '{print $1}')
echo "Commit 1 Hash: $COMMIT1_HASH"

"$RIG_BIN" lock file.txt
echo "Content Rev 2" > file.txt
"$RIG_BIN" add file.txt
"$RIG_BIN" commit -m "Rev 2"
"$RIG_BIN" push
COMMIT2_HASH=$("$RIG_BIN" log | grep "Rev 2" | awk '{print $1}')
echo "Commit 2 Hash: $COMMIT2_HASH"

# 2. Test pulling specific revision with # notation
echo -e "\n-> Testing: rig pull file.txt#1"
"$RIG_BIN" pull "file.txt#1"

if [ -f "file.txt@1" ]; then
    echo "Success: file.txt@1 exists."
    CONTENT=$(cat "file.txt@1")
    if [ "$CONTENT" == "Content Rev 1" ]; then
        echo "Success: Content is correct for Rev 1."
    else
        echo "Error: Content mismatch! Got: '$CONTENT'"
        exit 1
    fi
else
    echo "Error: file.txt@1 was not created!"
    exit 1
fi

# 3. Test pulling with positional argument
echo -e "\n-> Testing: rig pull file.txt 1"
"$RIG_BIN" pull file.txt 1

if [ -f "file.txt@1" ]; then
    echo "Success: file.txt@1 exists after positional pull."
fi

# 4. Test pulling by commit with @ notation
echo -e "\n-> Testing: rig pull file.txt@$COMMIT1_HASH"
"$RIG_BIN" pull "file.txt@$COMMIT1_HASH"
if [ -f "file.txt@1" ]; then
    echo "Success: file.txt@1 exists from commit pull."
fi

# 5. Test pulling all at commit
echo -e "\n-> Testing: rig pull * @$COMMIT1_HASH"
mkdir -p restore_dir
"$RIG_BIN" pull "*" "@$COMMIT1_HASH" --out restore_dir
if [ -f "restore_dir/file.txt" ]; then
    echo "Success: restore_dir/file.txt exists from commit * pull."
    CONTENT=$(cat "restore_dir/file.txt")
    if [ "$CONTENT" == "Content Rev 1" ]; then
        echo "Success: Content is correct for Rev 1 in restore_dir."
    fi
fi

echo -e "\n=== Pull Revision/Commit Test Completed Successfully! ==="
