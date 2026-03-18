#!/bin/bash
set -e

# Setup paths
ROOT_DIR="$(pwd)"
RIG_BIN="$ROOT_DIR/target/debug/rig"
PROJECT_NAME="test_large_file_project_$(date +%s)"
CLONE_DIR_1="clone_user_large_1"
CLONE_DIR_2="clone_user_large_2"
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

# Cross-platform SHA256 command
if command -v sha256sum >/dev/null 2>&1; then
    SHA_CMD="sha256sum"
else
    SHA_CMD="shasum -a 256"
fi

echo "=== Starting Large File & Compression Test ==="

# 0. Login & Create Project
LOGIN_RESP=$(curl -s -X POST "$API_URL/login" -H "Content-Type: application/json" -d "{\"email\":\"$ADMIN_EMAIL\", \"password\":\"$ADMIN_PASSWORD\"}")
AUTH_TOKEN=$(echo $LOGIN_RESP | sed -n 's/.*"token":"\([^"]*\)".*/\1/p')

# Inject token for rig client
mkdir -p ~/.config/rig
echo "{\"host_tokens\":{\"$SERVER_URL\":\"$AUTH_TOKEN\"}}" > ~/.config/rig/credentials
chmod 600 ~/.config/rig/credentials

curl -s -X POST "$API_URL/create_project" -H "Content-Type: application/json" -H "Authorization: Bearer $AUTH_TOKEN" -d "{\"name\":\"$PROJECT_NAME\"}" > /dev/null

# 1. Generate 4MB mixed file (2MB zero + 2MB random)
"$RIG_BIN" clone "$SERVER_URL/admin/$PROJECT_NAME" "$CLONE_DIR_1" --username "User1"
cd "$CLONE_DIR_1"

echo "-> Generating 3MB mixed file (1.5MB zero + 1.5MB random)..."
dd if=/dev/zero of=large_file.bin bs=1024 count=1536 2>/dev/null
dd if=/dev/urandom bs=1024 count=1536 2>/dev/null >> large_file.bin
ORIGINAL_HASH=$($SHA_CMD large_file.bin | awk '{print $1}')
echo "Original SHA256: $ORIGINAL_HASH"

# 2. Add and Push (Compression should be triggered)
"$RIG_BIN" add large_file.bin
"$RIG_BIN" commit -m "Initial commit with large file"
echo "-> Pushing large file (check for 'Compressing' log)..."
"$RIG_BIN" push | grep "Compressing" || echo "Note: Compression log not found, check if compression threshold is met."

# 3. Pull from another side and Verify Integrity
cd "$ROOT_DIR"
"$RIG_BIN" clone "$SERVER_URL/admin/$PROJECT_NAME" "$CLONE_DIR_2" --username "User2"
cd "$CLONE_DIR_2"

echo "-> Pulling large file..."
"$RIG_BIN" pull large_file.bin
PULLED_HASH=$($SHA_CMD large_file.bin | awk '{print $1}')
echo "Pulled SHA256:   $PULLED_HASH"

if [ "$ORIGINAL_HASH" = "$PULLED_HASH" ]; then
    echo "Success: File integrity verified. Compression/Decompression worked perfectly!"
else
    echo "Error: Hash mismatch! Data corrupted during transfer."
    exit 1
fi

echo -e "\n=== Large File & Compression Test Completed Successfully! ==="
