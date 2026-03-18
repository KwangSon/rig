# Data System Architecture

This document outlines the underlying data storage and synchronization architecture of Rig, detailing the structure and operational paradigms for both the client and the server.

## Client-Server Symmetry

The Rig architecture is built upon a symmetric metadata model. The core metadata tracking and internal index organizations are virtually identical across both the client and the server. This identical layout ensures consistency and simplifies the synchronization logic. However, the physical storage of binary data differs: the server maintains a multi-revision object store in `artifacts/`, while the client only stores the current working revision of a binary in the user's workspace to save disk space.

## Comparison with the Git File System

Rig's underlying file system takes heavy inspiration from Git but introduces several critical architectural differences explicitly designed to support collaborative design workflows and large binary assets:

### 1. Granular Revision Tracking
Unlike Git, which uses a Merkle tree to track snapshots of the entire repository state for every commit, Rig maintains **per-file revision and tracking information**. Each individual artifact (file) has its own distinct revision history, locking state, and metadata. This granular tracking enables efficient parallel collaboration and specific file-level rollbacks without needing to check out the entire project tree.

### 2. Lazy Loading (Zero-Byte Files)
To prevent local disk bloat and handle large assets efficiently (such as high-resolution images, 3D models, or large binary files), Rig employs a lazy-loading mechanism (similar to a sparse checkout). 
When a repository is initially cloned or fetched:
- The metadata and index are downloaded.
- The actual physical files manifest as **0-byte placeholders** in the working directory.
- The real file payload is downloaded and populated *only* when an explicit `pull` is executed on that specific artifact or directory.

### 3. Selective Synchronization (Partial Pull)
A major departure from Git's "all-or-nothing" clone/pull requirement is Rig's ability to fetch data à la carte. Because file payloads are decoupled from the core tree metadata, users can choose to download only the specific files or directories they need to work on (`rig pull <specific_path>`). This drastically reduces network bandwidth and local disk usage for massive projects containing hundreds of gigabytes of assets, as you are never forced to download files you don't intend to use.

### 4. Resumable Chunked Downloading
When a `pull` command is executed on very large assets, Rig prevents failures or bottlenecks by transmitting the data in **compressed chunks** rather than a single monolithic stream. 
This means if an unstable network connection drops—or if a user hits "pause/stop" mid-download—the system keeps track of the chunks already successfully written locally. When the `pull` is restarted, Rig seamlessly resumes from the exact chunk offset where it left off, rather than restarting the massive download from 0%.

## Client-Side Storage Structure

The client's `.rig/` directory is always located at the **project root**, following the same convention as Git's `.git/`. This is a structural requirement — it guarantees that `.rig/` and all working directory files always reside on the same filesystem, which is required for atomic file operations.

```text
my-project/
├── .rig/
│   ├── index                        ← Working directory state (JSON, atomically written)
│   ├── config                       ← Local configuration containing project, server_url, username
│   ├── HEAD                         ← Current branch pointer (e.g., "ref: refs/heads/main")
│   ├── refs/
│   │   └── heads/
│   │       └── main                 ← Commit UUID of the latest commit on this branch
│   └── objects/
│       ├── <commit-uuid>            ← Local commit records (one JSON file per commit)
│       └── ...
│
├── assets/
│   ├── weapon.png                   ← Real binary (pulled, rw- if locked / r-- otherwise)
│   ├── character.png                ← 0-byte placeholder (not yet pulled, r--)
│   └── background.png               ← Real binary (pulled, r--)
```

---

### Design Philosophy: Server as the Single Source of Truth

The client does **not** maintain a local binary object cache. The working directory file is the **only** local copy of the binary:

- All revision history is owned exclusively by the server's `artifacts/` directory.
- If a working directory file is deleted after a successful `push`, it can be restored via `rig pull`.
- If a working directory file is deleted after a local `commit` but before `push`, the data is unrecoverable locally. The user must discard the local commit and re-edit. This is an accepted tradeoff — `push` is expected to follow `commit` promptly.

---

### `.rig/index` Format

The index tracks the current state of every artifact in the working directory. It is the single source of truth for local state. **File permissions (`r--` / `rw-`) are a UX hint only** — all actual access control decisions are made against the index `lock` state, not the filesystem permission.

```json
{
  "version": 1,
  "artifacts": {
    "assets/weapon.png": {
      "artifact_id": "550e8400-e29b-41d4-a716-446655440000",
      "revision": 3,
      "local_state": "ready",
      "stage": "none",
      "locked": true,
      "lock_owner": "kwang",
      "lock_generation": "uuid",
      "staged": {
        "mtime": 1234567890,
        "size": 4096000
      }
    },
    "assets/character.png": {
      "artifact_id": "661f9511-f30c-52e5-b827-557766551111",
      "revision": 1,
      "local_state": "placeholder",
      "stage": "none",
      "locked": false,
      "lock_owner": null,
      "lock_generation": null,
      "staged": null
    }
  },
  "git_modules": {}
}
```

**Field definitions:**

| Field | Description |
| `version` | Schema version for future migration |
| `artifact_id` | Immutable UUID v4, generated once at `rig add` time and permanently retained. Never derived from the file path — this ensures locks and history survive `rig mv` |
| `revision` | Server-side revision number. Incremented **only on successful `rig push`** by the server. Local commits do not change this value |
| `local_state` | `"placeholder"` = 0-byte file, not yet pulled. `"ready"` = real binary present |
| `stage` | `"none"` = not staged. `"staged"` = staged via `rig add`, awaiting commit |
| `locked` | Whether this client holds an active lock on this artifact |
| `lock_owner` | Username of the lock holder |
| `lock_generation` | UUID token from the server's `file_locks` table, used for push-time validation |
| `staged.mtime` | File modification time (UTC epoch seconds) recorded at `rig add` |
| `staged.size` | File size in bytes recorded at `rig add` |
| `git_modules` | Maps subdirectories to Git repositories to be managed by `rig status/clone/push` (e.g. tracking `client/` inside the parent repository). |

**`staged` field lifecycle:**
- Set by `rig add` only.
- Never modified after `rig add` — not by `rig commit`, not by any file modification.
- Remains as the comparison baseline until the next `rig add`.
- Reset to `null` after a successful `rig push`.

---

### `.rig/config` Format

Provides the local configuration for the repository, determining where it pushes to and the user identity. It is formatted in JSON.

```json
{
  "project": "myho",
  "server_url": "http://localhost:3000",
  "username": "mm"
}
```

| Field | Description |
|---|---|
| `project` | The logical project name recognized by the server. |
| `server_url` | The base API URL or SSH URL where Rig connects for fetching/pushing. |
| `username` | The username associated with the current clone, used as the default author for new commits. |

---

### `.rig/HEAD` and `.rig/refs/` Format

Just like Git, Rig maintains explicit branch references. The branch and head state are extracted from the monolithic index and stored in individual text files.

*   `HEAD`: Contains a pointer to the current branch, e.g., `ref: refs/heads/main`.
*   `refs/heads/<branch>`: Contains the plain text UUID of the latest commit on that specific branch.

When a new commit is created, the UUID in the appropriate `refs/heads/` file is updated.

---

### `.rig/objects/<commit-uuid>` Format

Each local commit is stored as a single JSON file named by its UUID. The `parent` field forms a linked chain for ordering. Timestamps are UTC epoch seconds.

```json
{
  "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "parent": "previous-commit-uuid-or-null",
  "artifacts": [
    {
      "path": "assets/weapon.png",
      "artifact_id": "550e8400-e29b-41d4-a716-446655440000",
      "revision_base": 3,
      "hash": "blake3:4a8d9f2e...",
      "op": "upsert"
    },
    {
      "path": "assets/old_icon.png",
      "artifact_id": "772fa622-a41d-63f6-c938-668877662222",
      "revision_base": 2,
      "hash": null,
      "op": "delete"
    }
  ],
  "message": "Update hero image, remove old icon",
  "author": "kwang",
  "timestamp": 1234567890
}
```

**Field definitions:**

| Field | Description |
|---|---|
| `id` | UUID v4, unique per commit |
| `parent` | UUID of the previous commit. `null` for the first commit after a push |
| `artifacts` | List of artifact changes in this commit. Must contain at least one entry — empty commits are rejected |
| `path` | Logical file path at commit time |
| `artifact_id` | Immutable artifact identifier |
| `revision_base` | The server revision this edit is based on. The server hard-rejects the push if `server_revision != revision_base` (Stale Lineage) |
| `hash` | blake3 hash of the binary at commit time. `null` for `delete` operations |
| `op` | `"upsert"` for new or modified files. `"delete"` for removals |
| `message` | Commit message |
| `author` | Taken from `~/.gitconfig` `[user].name` |
| `timestamp` | UTC epoch seconds |

---

### File Change Detection Strategy

Rig uses a three-stage detection strategy that balances speed with correctness:

**Stage 1 — `rig add`**
Records `mtime` and `size` into the index `staged` field. No hash is computed. This keeps `add` fast even for multi-gigabyte assets.

**Stage 2 — `rig commit`**
Computes a blake3 hash of the file **twice in succession** (double-hash) to detect writes that occur during hashing (TOCTOU):
```
hash1 = blake3(file)
hash2 = blake3(file)
if hash1 != hash2 → abort, prompt user to retry
if hash1 == hash2 → store hash in commit record
```
If the commit is aborted, the user must re-run `rig add`.

**Stage 3 — `rig push` pre-flight**
Reuses the hash stored at commit time rather than recomputing it. Before transmitting, a fast pre-flight check determines whether the file has changed since commit:
```
if size != staged.size:
    recompute hash
else if mtime != staged.mtime:
    recompute hash
else if (now - staged.mtime) < 1 second:
    recompute hash          ← guards against low-resolution FS timestamps
else:
    use commit hash as-is

if recomputed hash != commit hash:
    ERROR: File changed after commit. Re-run 'rig add' and 'rig commit'.
```

---

### Server-Side Upload Integrity Verification

The server independently verifies every binary payload received during `rig push`:

1. Receives the complete binary upload.
2. Computes a blake3 hash of the received data.
3. Compares against the `hash` field transmitted in the push payload.
4. If hashes do not match → **hard-reject**. Catches in-transit corruption and client bugs.
5. Verifies `revision_base` against the server's current `HEAD` for that artifact. If they differ → **hard-reject** (Stale Lineage).
6. Only after both checks pass does the server write to `artifacts/[artifact_id]/rev_N`, increment the revision, and update the index.

---

### Strict State Machine: Preventing Invalid States

The client rejects any operation that would produce an ambiguous or unrecoverable state:

| Attempted Action | Condition | Client Response |
|---|---|---|
| `rig add` | No active lock on this file (per index) | `ERROR: File is read-only. Use 'rig lock' first.` |
| `rig add` | `local_state` is `"placeholder"` | `ERROR: File not downloaded. Run 'rig pull' first.` |
| `rig commit` | No staged artifacts exist | `ERROR: Nothing to commit. Stage changes with 'rig add' first.` |
| `rig commit` | double-hash mismatch | `ERROR: File changed during commit. Retry.` |
| `rig commit` | Staged file missing from working directory | `ERROR: Staged file not found. File may have been deleted.` |
| `rig pull` | Active local lock on target (per index) | `ERROR: File is locked locally. Push or unlock before pulling.` |
| `rig push` | Remote revision differs from base | `ERROR: Remote updated. Run 'rig pull' first.` |
| `rig checkout` | Uncommitted local changes exist | `ERROR: Uncommitted changes. Commit or stash first.` |

The only exception to the lock requirement is **new file creation**. A file that does not yet exist in the remote index can be added and pushed without a lock. The server enforces a Namespace Collision Check at push time and hard-rejects if the path is already mapped to an existing artifact.

---

### Atomic Write Rules

All writes to `.rig/` must be atomic to prevent corruption if the process is interrupted:

**index write:**
```
write → .rig/index.tmp
fsync
rename(.rig/index.tmp → .rig/index)
```

**commit write:**
```
write → .rig/objects/<uuid>.tmp
fsync
rename(.rig/objects/<uuid>.tmp → .rig/objects/<uuid>)
```

**pull / download write:**
```
write chunks → .rig/tmp/<artifact_id>.part   ← resumable
checksum verify
rename(.rig/tmp/<artifact_id>.part → working directory file)
```

---

### Unpushed Commit Warning

`rig status` always surfaces a warning when `.rig/objects/` contains uncommitted JSON files:

```
⚠ Unpushed commits detected — local data is NOT backed up until pushed.
  Deleting working directory files before pushing will permanently lose data.
  → Run 'rig push' to back up your changes to the server.
```

This warning is also shown at the start of `rig lock` and `rig checkout` if unpushed commits are detected.

## Server-Side Storage Structure

On the server, data is partitioned and isolated by `project_id`. This multi-tenant design ensures that all files and metadata are strictly scoped to their respective projects. 

The internal storage layout for a specific project looks like this:

```text
[project_id]/
 ├── .rig/         # Core metadata, configuration, index, and commit history
 └── artifacts/    # The physical object store for actual file data, strictly keyed by artifact ID
      ├── [artifact_id_1]/
      │    ├── rev_1
      │    └── rev_2
      └── [artifact_id_2]/
           └── rev_1
```

### The `.rig/` Directory
This directory serves the exact same purpose as it does on the client. It contains all the structural metadata for the project, including:
- `index`: Functions similarly to Git's Staging Area (`.git/index`), but extended for Rig. It maps logical file paths (e.g., `assets/weapon.png`) to their immutable `artifact_id`, size, current revisions, locking states, and tracks lazy-load data availability.
- `config`: Functions identically to Git's local configuration (`.git/config`). It is a project-specific configuration containing remote server mappings (`[remote]`), user profiles, and branch schemas.
- **Commit/Revision Graph**: Maintains the global history pointer of what changed and when.

### The `artifacts/` Directory
This directory acts as the object store mechanism for the project, completely decoupled from string-based logical file paths. While the `.rig/index` knows *about* the file names, the `artifacts/` directory holds the concrete binary data organized purely by continuous identifiers.
- **Artifact-ID Grouping:** Each tracked file has a dedicated sub-directory named after its immutable `artifact_id` (e.g., `artifacts/e3b0c442../`).
- **Revision Blobs:** Inside each `artifact_id` folder, physical binary data is stored and designated by its revision history (e.g., `rev_1`, `rev_2`). This allows for instantaneous per-file rollbacks without traversing a repository-wide snapshot tree.
- These are the "real" file payloads that are transferred to users when they perform a `pull` request. For example, `rig pull character.png@rev_1` simply reaches into `artifacts/[linked_artifact_id]/rev_1` and streams that specific payload chunk to the client.

## 3. Authentication and Author Identity

To interact with the remote server safely and manage author metadata without reinventing the wheel, Rig employs a hybrid approach blending native Git paradigms with robust security standards.

### Identity Sourcing (`.gitconfig`)
For local commits and author identity tracking, Rig **automatically inherits your global `~/.gitconfig` configurations**. When you create a local `rig commit` or `rig push` to the server, Rig reads the `[user]` section (specifically `name` and `email`) from your `~/.gitconfig` and attaches these values to your Rig commits. 
This means you do not need to configure an author name specifically for Rig; your existing Git identity translates seamlessly.

### Secure Remote Operations (SSH Keys)
When performing network operations like `pull` or `push`, relying on HTTP basic authentication (usernames and passwords) over arbitrary network requests is considered a security risk.
Instead, **Rig requires SSH key-based authentication for remote operations**.
1. **Pre-registration:** Users must first create a web account on the Rig server.
2. **Key Registration:** Before executing any `rig clone`, `pull`, or `push` command, the user must upload their public SSH key to the Rig server dashboard.
3. **Execution:** All rigorous network operations authorize the client by verifying their local private SSH key against the registered public key (`ssh_keys` database table). 

This ensures password-less, cryptographically secure collaboration.

## 4. Role-Based Access Control (RBAC) & Permissions

As defined in the `permissions` table, Rig enforces strict Role-Based Access Control. Because Rig relies on exclusive locks (changing file permissions from `r--` to `rw-`), concurrent modification conflicts are structurally impossible. However, this introduces the need for robust access and override management:

- **Read Access (`read`)**: Users with this level can only perform non-mutating operations: `clone`, `fetch`, `pull`, and `log`. They cannot acquire locks.
- **Write Access (`write`)**: In addition to read privileges, these users can perform mutating operations on the repository: `lock`, `add`, `commit`, and `push`.
- **Admin Access (`admin`)**: Admins have full control over the project. Crucially, because there are no timeouts on acquired locks, if a user acquires a lock and becomes unavailable (e.g., leaves the company or goes on vacation), an Admin must intervene. **Only users with the `admin` access level in the `permissions` table can execute `rig unlock <path> --force` to forcibly seize and release a lock** held by another user.

### Mitigating Concurrency with Offline Commits
While exclusive locks prevent simultaneous online edits, an architectural challenge arises when an offline user holds a lock on a binary asset, continues to create local `commit`s, but an Admin forces an unlock (`--force`). If another user acquires the freed lock and pushes, the offline user's subsequent push would cause an un-mergeable binary conflict. 

To structurally prevent this data loss while preserving the ability to create offline commits (for branching and stashing), Rig enforces **Push-Time Lock Validation (Revocation Tokens & Lineage):**
- **Lock Generation ID:** The `file_locks` table tracks a generation ID (or token) for every lock. 
- **Validation on Push (Modifications):** When a client executes `rig push` to modify an existing `artifact_id`, it transmits the Lock Generation ID it held at the time of the commits, as well as the **Base Revision Hash** it started editing from.
- **Hard Rejection (Authorization):** The server strictly compares the client's token against the current database state. If the server's lock state has mutated (e.g., forced unlocked and re-acquired by someone else), the server **hard-rejects the push**. The offline user's data remains safe locally, but they are prevented from corrupting the master server state.
- **Hard Rejection (Stale Lineage):** To prevent regression, the server verifies the client's Base Revision Hash. If the client locked and edited an outdated revision (i.e., the base hash does not match the server's current `HEAD` for that artifact), the push is **hard-rejected**. 
- **Namespace Collision Check (Ghost Creation Prevention):** If a client pushes a "new" file (e.g., bypassing `rig lock` and manually creating an asset over a 0-byte placeholder), the server must independently query its own `.rig/index` for that branch. If the logical path already maps to an existing `artifact_id`, the server **MUST demand a valid lock token** and hard-reject the push if none is provided. A "new file" push is only accepted if the logical path is genuinely unregistered.
- **Server-Side Deletion Validation (Unlocked Deletion Prevention):** Removing a tracked file from the project is a terminal modification. When the server receives a new index tree during `rig push`, it computes a diff against the current branch index. For any `artifact_id` that is being **removed or unmapped**, the server **MUST demand a valid lock token** for that `artifact_id`. If a user attempts to delete a file locked by someone else (or a file they haven't locked themselves), the push is **hard-rejected**.

## 5. Source Code vs. Asset Segregation

A core philosophy of Rig is that it does **not** attempt to replace Git for source code version control. Text-based source code (e.g., `.rs`, `.py`, `.js`, `.cpp`) relies heavily on line-by-line diffing, auto-merging, and branching—features optimized for Git. 

If a user attempts to run `rig add` on source code files, the Rig client will explicitly emit a warning.
The intended workspace architecture for a project is:
- **Binary Assets & Large Files**: Tracked natively by Rig's granular, lock-based `.rig/index`.
- **Source Code**: Tracked in standard Git repositories, which are then mounted into the Rig workspace using the `rig gitmodule` system.

This ensures that artists/designers get the binary-locking UX they need, while software engineers retain the standard Git tooling they expect, gracefully fused in one contiguous workspace directory.

## 6. DB schema

The server relies on a PostgreSQL database (`postgresql://kwang@localhost/rig`) to manage system state, authentication, and access control. 
The core tables are:

### `users`
Manages system users and authentication credentials.
- `id` (UUID, Primary Key)
- `name` (Text)
- `email` (Text, Unique)
- `password_hash` (Text)
- `created_at` (Timestamp)

### `projects`
Stores high-level repository/project information.
- `id` (UUID, Primary Key)
- `name` (Text, Unique)
- `owner_id` (UUID, Foreign Key → `users.id`)
- `created_at` (Timestamp)

### `permissions`
Provides Role-Based Access Control (RBAC) tying users to projects.
- `id` (UUID, Primary Key)
- `user_id` (UUID, Foreign Key → `users.id`)
- `project_id` (UUID, Foreign Key → `projects.id`)
- `access` (Text: 'read', 'write', or 'admin')

### `ssh_keys`
Stores public SSH keys for secure artifact and git submodule access management.
- `id` (UUID, Primary Key)
- `user_id` (UUID, Foreign Key → `users.id`)
- `title` (String)
- `key_data` (Text)
- `created_at` (Timestamp)

### `file_locks`
Tracks explicit, granular file locks to prevent concurrent modifications on individual artifacts. Locks are tied to the immutable `artifact_id` rather than a mutable string path, ensuring locks remain valid even if the file is moved or renamed locally via `rig mv`. Furthermore, locks are **branch-isolated**, meaning a lock is granted for a specific artifact *on a specific branch*.
- `id` (UUID, Primary Key)
- `project_id` (UUID, Foreign Key → `projects.id`)
- `artifact_id` (String)
- `branch` (String)
- `locked_by` (UUID, Foreign Key → `users.id`)
- `lock_generation_id` (UUID, Auto-generated token for push-time validation)
- `locked_at` (Timestamp)
- `updated_at` (Timestamp)
