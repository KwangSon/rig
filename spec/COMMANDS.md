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
- **Description**: Acquires an exclusive lock to prevent other users from concurrently editing the artifact.
- **Specification & Side-effects**:
  - 🔒 **Permission Change**: Upon successfully acquiring the lock from the server, the local file system permissions for the target file are changed from **read-only (`r--`) to read/write (`rw-`)**.
  - **Automatic Data Fetch**: If the targeted artifact is currently a 0-byte placeholder (lazy-loaded state), `rig lock` will automatically trigger a background `pull` to fetch the real file payload before granting `rw-` access.
  - If another user already holds the lock, the server will deny the request, and the local file will remain read-only.
  - This command must precede any file modification or the use of the `add` command.

### `rig unlock <path> [--force]`
- **Description**: Releases the lock on an artifact after editing is complete, allowing other users to acquire a lock and modify it.
- **Specification & Side-effects**:
  - 🔓 **Permission Change**: Upon successful unlock, the local file permissions are **reverted to read-only (`r--`)** to prevent further unauthorized modifications.
  - The `--force` (`-f`) flag forcibly breaks a lock held by another user. Note that this may fail depending on permission controls or server settings.
  - It is highly recommended to commit and push local changes before unlocking. Uncommitted changes might be overwritten by other users if the lock is released prematurely.

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
