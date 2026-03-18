# Rig Handbook: Workflows & Examples

This document provides practical, step-by-step examples of how to combine `rig` commands for common day-to-day workflows. 

---

## 1. Getting Started: Cloning a Repository
To begin working on an existing Rig project, you must first clone it to your local machine.

```bash
# Clone the repository
# Format: rig clone http://<server>/<username>/<project>
rig clone http://localhost:3000/kwang/my-project

# Move into the newly created directory
cd my-project
```
*(Note: To save disk space, all cloned files are initially downloaded as **0-byte read-only placeholders**. The actual file data is fetched when you pull or lock them.)*

---

## 2. Workflow: Creating and Pushing a New File
When you create a completely **new** file that does not yet exist in the remote repository, you do not need to lock it initially. 

```bash
# 1. Create your new asset or file
touch new_design.png

# 2. Stage the new file in Rig's index
rig add new_design.png

# 3. Create a local commit
rig commit --message "Add new design for the homepage"

# 4. Push the commit to the remote server
rig push
```

---

## 3. Workflow: Modifying an Existing File (The Lock Pattern)
Because Rig uses an explicit locking system for collaborative design and asset management, modifying an already tracked file requires a specific flow. **You must acquire a lock before you start editing.**

```bash
# 1. Lock the file to gain exclusive write access. 
# This tells the server you are working on it. If it is a 0-byte placeholder,
# Rig will automatically fetch the real file data and change permissions to `rw-`.
rig lock existing_design.png

# 2. Open the file in your preferred software and make your edits.
# ... editing in progress ...

# 3. Stage the modified file
rig add existing_design.png

# 4. Create a local commit
rig commit --message "Update the hero image in the existing design"

# 5. Push the new revision to the server
rig push

# 6. Unlock the artifact so your teammates can edit it in the future.
# This reverts the local file permission back to read-only `r--`.
rig unlock existing_design.png
```

---

## 4. Workflow: Syncing and Updating
When collaborating with others, you will frequently need to pull their changes down to your local workspace. Because Rig uses lazy-loading, pulling is how you actually download the file bytes.

**Important**: You cannot `pull` an asset that you currently have **locked**. A locked file is protected from being overwritten by external changes. You must `unlock` the file first if you wish to revert your changes to the server version.

```bash
# Option A: Check what has changed on the server without downloading heavy files
rig fetch

# Option B: Pull the actual file data for a specific asset (fails if file is locked)
rig pull existing_design.png

# Option C: Pull all updated files in the current directory (skips or fails on locked files)
rig pull "*"
```

---

## 5. Workflow: Context Switching (Branches & Stashing)
If you need to experiment or work on multiple features simultaneously without disrupting the main project state.

```bash
# 1. Create a new branch for your experimental work
rig branch experiment-dark-mode

# 2. Switch your workspace to the new branch
rig checkout experiment-dark-mode

# ... lock, edit, add, and commit your work ...

# 3. You need to quickly fix something on the main branch,
# but you have uncommitted changes. Rig will intentionally block you 
# from checking out to prevent data loss. You must stash them safely first!
rig stash

# 4. Now the workspace is clean. Safely switch back to the main branch.
rig checkout main
```

---

## 6. Workflow: Using External Git Modules
Rig can track snapshots of standard Git repositories within its ecosystem.

```bash
# 1. Add an external Git dependency
rig gitmodule add https://github.com/example/repo ./libs/repo

# 2. When you clone a Rig repository, or pull a new Git module configuration 
# created by a teammate, you MUST sync to physically clone the git repositories.
rig gitmodule sync
```
