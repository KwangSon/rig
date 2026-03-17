# Rig CLI Command Specification and Guidelines

This document outlines the complete specifications, operational patterns, and side-effects (including file system permission changes) for the commands supported by the `rig` client.


| Command | Description | Read-only (`r--`) | Read/Write (`rw-`) |
| --- | --- | --- | --- |
| `rig clone` | Clones a repository, metadata, and initializes the workspace | Yes (Default state) | No |
| `rig lock` | Gains exclusive write access and changes file permissions | No | Yes |
| `rig unlock` | Releases an exclusive lock and reverts file permissions | Yes | No |
| `rig add` | Stages a modified or new artifact into the index | - | - |
| `rig commit` | Records local changes into a new local revision | - | - |
| `rig push` | Uploads local revisions to the remote server | - | - |
| `rig pull` | Downloads file data for specific artifacts | Yes (unless locked) | - |
| `rig fetch` | Updates remote metadata without downloading data | - | - |
| `rig status` | Displays working directory changes and lock states | - | - |
| `rig log` | Shows history and commit logs for the repository/file | - | - |
| `rig branch` | Lists, creates, or deletes local branches | - | - |
| `rig checkout`| Switches the workspace to another branch | - | - |
| `rig stash` | Saves uncommitted changes temporarily | - | - |
| `rig mv` | Moves or renames tracked artifacts | - | - |
| `rig gitmodule`| Manages external git repository references | - | - |

---

## Permission and Lock Management

The Rig system inherently uses an explicit Lock/Unlock mechanism for version control in collaborative scenarios.

### `rig lock <path>`
- **Description**: Acquires an exclusive lock to prevent other users from concurrently editing the artifact on the current branch.
- **Specification & Side-effects**:
  - 🔒 **Permission Change**: Upon successfully acquiring the branch-isolated lock from the server (bound securely to `artifact_id` and the current branch name), the local file system permissions for the target file are changed from **read-only (`r--`) to read/write (`rw-`)**.
  - **Cross-Branch Warning**: Because locks are isolated per branch, the server will check if the artifact is already locked on *any other branch*. If it is, the server grants the lock but the client MUST display a prominent warning: "WARNING: This binary artifact is currently locked and being edited on another branch. Binary files cannot be merged. Parallel edits will result in un-mergeable conflicts across branches." This ensures the user is deliberately accepting the risk of a parallel branch edit.
  - **Outdated File Guardrail**: If the local file's revision is older than the server's current `HEAD`, **the server MUST deny the lock request** and return an error prompting the user to `rig pull` first. The client MAY additionally surface this as a pre-flight warning before contacting the server, but this does not substitute the server-side enforcement.
  - **Automatic Data Fetch**: If the targeted artifact is currently a 0-byte placeholder (lazy-loaded state), `rig lock` will automatically trigger a background `pull` to fetch the real file payload before granting `rw-` access.
  - If another user already holds the lock, the server will deny the request, and the local file will remain read-only.
  - This command must precede any file modification or the use of the `add` command.

### `rig unlock <path> [--force]`
- **Description**: Releases the branch-specific lock on an artifact after editing is complete, allowing other users to acquire a lock on this branch and modify it.
- **Specification & Side-effects**:
  - 🔓 **Permission Change**: Upon successful unlock, the local file permissions are **reverted to read-only (`r--`)** to prevent further unauthorized modifications.
  - **Local Status Check (Enforce Push on Unlock)**: Before releasing the lock, the client deeply verifies the local `.rig/` index *and* the local **stash stack** (`rig stash`) for the currently active branch. If the artifact has pending local commits OR unpopped stashed modifications, the command **aborts with a hard error**. Because stashing binary edits hides them from the active index, checking the stash stack is mandatory to prevent un-mergeable conflicts upon subsequent popping.
  - The `--force` (`-f`) flag forcibly breaks a branch-isolated lock held by another user from the server-side. Note that this may fail depending on permission controls or server settings. An offline user who has their lock forcefully revoked will have their subsequent `push` rejected due to Lock Generation ID mismatch.

---

## Core Commands

### `rig clone <url> [path] [--username <username>]`
- **Description**: Clones a Rig remote repository from the specified URL into a local directory.
- **Specification & Side-effects**:
  - Initializes the repository structure (`.rig/`), metadata, and files.
  - **Lazy Loading**: Cloned artifact payloads are downloaded as **0-byte placeholders** by default. Use `pull` to fetch the real data.
  - Downloaded placeholder files are created in a write-protected state (`r--`) by default. Use `lock` to modify them.
  - If `path` is omitted, an automatic directory based on the repository name from the URL will be created.

### `rig add <path>`
- **Description**: Adds a new artifact to the index or stages modifications of an existing artifact.
- **Specification & Side-effects**:
  - Successfully executing this command may require the target artifact to be **locked**.

### `rig commit --message <message>`
- **Description**: Records added or modified files as a new commit in the local repository.
- **Specification & Side-effects**:
  - **Offline Operation**: This command records the revision solely within the local `.rig/` system. It does not transmit changes to the remote server (`push`).

### `rig push [--message <message>]`
- **Description**: Uploads local commits and changes to the remote server.
- **Specification & Side-effects**:
  - If the `--message` parameter is omitted, the most recent local commit message is used automatically.
  - **Push-Time Lock Validation**: The client transmits its known Lock Generation ID and the **Base Revision Hash** it started editing from for modified artifacts. The server **hard-rejects the push** if the lock token does not match the server's current state (Authorization Failure) or if the Base Revision Hash does not match the server's current `HEAD` (Stale Lineage/Regression Prevention).
  - Creates a new revision on the server. To avoid synchronization conflicts, it is recommended to `pull` the latest changes before using `push`.

### `rig pull <path> [revision] [--out <out_path>]`
- **Description**: Fetches the latest (or specific) revision of a file/directory from the remote server and updates the local working directory.
- **Specification & Side-effects**:
  - The `path` parameter is flexible (e.g., `file.png`, `dir/`, `*`), and revisions can be specified inline like `file.png@10`.
  - The `--out` flag allows downloading to an alternate path to prevent overwriting existing files.
  - Local modifications that have not been stashed or committed risk being overwritten by the remote data.

### `rig fetch`
- **Description**: Connects to the remote server to update metadata only, without downloading file contents.
- **Specification & Side-effects**:
  - Useful for checking the commit status of the remote repository without altering the physical files in the local working directory.

---

## Status and History

### `rig status`
- **Description**: Displays the current status of the working directory, including modified files, lock states, and differences between local and remote states.
- **Specification & Side-effects**:
  - Frequently used to verify if modified files are properly staged (`add`) before committing.

### `rig log [path]`
- **Description**: Shows the commit history and change logs.
- **Specification & Side-effects**:
  - Specifying an optional `[path]` filters the output to show only the history relevant to that specific file or directory, rather than the entire repository.

---

## Branches and Workspace

### `rig branch [name]`
- **Description**: Lists local branches or creates/deletes a new branch if a name is provided.

### `rig checkout <branch>`
- **Description**: Switches the current working environment to the specified branch.
- **Specification & Side-effects**:
  - Changes the HEAD pointer and index. Files in the working directory are replaced to match the version of the target branch.
  - **Safe Switch**: If the working directory has uncommitted local changes that conflict with the target branch, the command will **abort with an error** to prevent data loss. The user must explicitly `commit` or `stash` changes before attempting the checkout again.

### `rig stash`
- **Description**: Temporarily stores uncommitted working modifications and reverts the working directory to a clean state.
- **Specification & Side-effects**:
  - **Remote Shelving Architecture Integration**: Supports creating a stash branch on the server-side to offload large files, optimizing disk efficiency and preventing local storage bloat.

### `rig mv <src> <dst>`
- **Description**: Moves or renames a tracked artifact.
- **Specification & Side-effects**:
  - Physically moves the file within the local file system and automatically updates the tracked path within the internal `rig` index.
  - **Collision Check**: Before executing the move, the client MUST check the local `.rig/index` to verify that `dst` is not already mapped to an existing `artifact_id`. If a collision is detected, the command MUST abort with an error: `Error: destination path 'dst' is already tracked as artifact [artifact_id]. Use 'rig lock' and remove or rename the destination artifact first.`
  - **Lock Continuity**: Because server-side locks are bound to the immutable `artifact_id` rather than the string path, `rig mv` can operate purely locally and offline without orphaning the server lock. The lock remains securely attached to the data payload regardless of its local path.

---

## Git Modules Management

Rig provides functionality to integrate snapshots of external Git projects outside of its own ecosystem.

### `rig gitmodule add <url> <path> [--commit <hash>]`
- Adds an external Git repository to a specific path. If `--commit` is omitted, it defaults to the remote HEAD.

### `rig gitmodule update <path> --commit <hash>`
- Changes the commit hash version tracked by an already added Git module.

### `rig gitmodule status`
- Displays a summarized list of all currently managed Git modules and their referenced commit states.

### `rig gitmodule sync`
- **Description**: Clones and checks out actual Git repositories based on the Git module declarations in the local `index` / `config`.
- **Specification & Side-effects**:
  - Execution of `sync` is mandatory to ensure a complete project tree after a fresh repository clone or after pulling external `gitmodule` configuration changes modified by others.
