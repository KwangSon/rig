#!/bin/bash
set -e

# Setup paths
# We'll use the current working directory for test folders
ROOT_DIR="$(pwd)"
RIG_BIN="$ROOT_DIR/target/debug/client"
PROJECT_NAME="full_workflow_test_project"
CLONE_DIR="full_workflow_cloned_project"
SERVER_URL="http://localhost:3000"

# Cleanup function to be called on EXIT
function cleanup {
    echo -e "\n--- Cleaning up test directories ---"
    cd "$ROOT_DIR"
    rm -rf "$PROJECT_NAME" "$CLONE_DIR"
}
trap cleanup EXIT

# Ensure binary exists
if [ ! -f "$RIG_BIN" ]; then
    echo "Error: Rig binary not found at $RIG_BIN. Please run 'cargo build' first."
    exit 1
fi

echo "=== Starting Full Workflow Test ==="

# 1. Initialize project
echo -e "\n--- 1. Initializing project: $PROJECT_NAME ---"
mkdir -p "$PROJECT_NAME"
cd "$PROJECT_NAME"
# Now prompts for server URL and username
printf "\nJone <jone@tt.com>\n" | "$RIG_BIN" init

# 2. Add and Push first revision
echo -e "\n--- 2. Adding first artifact ---"
echo "Revision 1 content" > file1.txt
"$RIG_BIN" add file1.txt
"$RIG_BIN" commit -m "Initial commit with file1.txt"
"$RIG_BIN" push

# 3. Clone project into a new directory
cd "$ROOT_DIR"
echo -e "\n--- 3. Cloning project into $CLONE_DIR ---"
printf "CloneUser\n" | "$RIG_BIN" clone "$SERVER_URL/$PROJECT_NAME" "$CLONE_DIR"

# 4. Pull and Modify from Clone
cd "$CLONE_DIR"
echo -e "\n--- 4. Pulling and modifying from clone ---"
"$RIG_BIN" pull file1.txt
echo "Original content in clone:"
cat file1.txt

"$RIG_BIN" lock file1.txt
echo "Revision 2 content from clone" >> file1.txt
"$RIG_BIN" commit -m "Revision 2 from cloned repository"
"$RIG_BIN" push

# 5. Verify History (Log & Blame)
echo -e "\n--- 5. Verifying history in clone ---"
"$RIG_BIN" log
echo ""
"$RIG_BIN" blame file1.txt

# 6. Synchronize Original Repository
cd "$ROOT_DIR/$PROJECT_NAME"
echo -e "\n--- 6. Synchronizing original repository ---"
"$RIG_BIN" pull file1.txt
echo "Final content in original repo:"
cat file1.txt

echo -e "\n=== Full Workflow Test Completed Successfully! ==="
