#!/bin/bash
set -e

# Setup paths
ROOT_DIR="$(pwd)"
RIG_BIN="$ROOT_DIR/target/debug/client"
PROJECT_NAME="test_force_unlock_project"
CLONE_DIR_1="clone_user_1"
CLONE_DIR_2="clone_user_2"
SERVER_URL="http://localhost:3000"

# Cleanup function
function cleanup {
    echo -e "\n--- Cleaning up test directories ---"
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

# 1. Initialize project
echo -e "\n--- 1. Initializing project: $PROJECT_NAME ---"
mkdir -p "$PROJECT_NAME"
cd "$PROJECT_NAME"
# 첫 번째 \n은 기본 서버 URL 수락, 두 번째는 사용자 이름 입력
printf "\nUser1\n" | "$RIG_BIN" init
echo "Initial content" > file.txt
"$RIG_BIN" add file.txt
"$RIG_BIN" commit -m "Initial commit"
"$RIG_BIN" push

# 2. Clone for User 1 and User 2
cd "$ROOT_DIR"
echo -e "\n--- 2. Cloning for User 1 and User 2 ---"
printf "User1\n" | "$RIG_BIN" clone "$SERVER_URL/$PROJECT_NAME" "$CLONE_DIR_1"
printf "User2\n" | "$RIG_BIN" clone "$SERVER_URL/$PROJECT_NAME" "$CLONE_DIR_2"

# 3. User 1 locks the file
cd "$ROOT_DIR/$CLONE_DIR_1"
echo -e "\n--- 3. User 1 locking file ---"
"$RIG_BIN" lock file.txt

# 4. User 2 tries to unlock it (should fail)
cd "$ROOT_DIR/$CLONE_DIR_2"
echo -e "\n--- 4. User 2 trying to unlock file (should fail) ---"
if "$RIG_BIN" unlock file.txt; then
    echo "Error: User 2 should NOT be able to unlock User 1's file without force"
    exit 1
else
    echo "Success: User 2 failed to unlock (as expected)"
fi

# 5. User 2 force unlocks it (should succeed)
echo -e "\n--- 5. User 2 force unlocking file (should succeed) ---"
"$RIG_BIN" unlock file.txt --force
echo "Success: User 2 force unlocked the file"

# 6. User 2 can now lock it
echo -e "\n--- 6. User 2 can now lock the file ---"
"$RIG_BIN" lock file.txt
echo "Success: User 2 locked the file after force unlock"

echo -e "\n=== Force Unlock Test Completed Successfully! ==="
