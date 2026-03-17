# Data System Architecture

This document outlines the underlying data storage and synchronization architecture of Rig, detailing the structure and operational paradigms for both the client and the server.

## Client-Server Symmetry

The Rig architecture is built upon a symmetric data model. The core storage structures, metadata tracking, and file organizations are virtually identical across both the client and the server. This identical layout ensures consistency, simplifies the synchronization logic, and provides a unified mental model for the entire repository system.

## Comparison with the Git File System

Rig's underlying file system takes heavy inspiration from Git but introduces two critical architectural differences explicitly designed to support collaborative design workflows and large binary assets:

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

As defined in the `permissions` and `users` tables, Rig enforces strict Role-Based Access Control. Because Rig relies on exclusive locks (changing file permissions from `r--` to `rw-`), concurrent modification conflicts are structurally impossible. However, this introduces the need for robust access and override management:

- **Read Access (`read`)**: Users with this level can only perform non-mutating operations: `clone`, `fetch`, `pull`, and `log`. They cannot acquire locks.
- **Write Access (`write`)**: In addition to read privileges, these users can perform mutating operations on the repository: `lock`, `add`, `commit`, and `push`.
- **Admin Access (`admin`)**: Admins have full control over the project. Crucially, because there are no timeouts on acquired locks, if a user acquires a lock and becomes unavailable (e.g., leaves the company or goes on vacation), an Admin must intervene. **Only users with the `admin` role can execute `rig unlock <path> --force` to forcibly seize and release a lock** held by another user.

### Mitigating Concurrency with Offline Commits
While exclusive locks prevent simultaneous online edits, an architectural challenge arises when an offline user holds a lock on a binary asset, continues to create local `commit`s, but an Admin forces an unlock (`--force`). If another user acquires the freed lock and pushes, the offline user's subsequent push would cause an un-mergeable binary conflict. 

To structurally prevent this data loss while preserving the ability to create offline commits (for branching and stashing), Rig enforces **Push-Time Lock Validation (Revocation Tokens):**
- **Lock Generation ID:** The `file_locks` table tracks a generation ID (or token) for every lock. 
- **Validation on Push:** When a client executes `rig push`, it transmits the Lock Generation ID it held at the time of the commits. 
- **Hard Rejection:** The server strictly compares the client's token against the current database state. If the server's lock state has mutated (e.g., forced unlocked and re-acquired by someone else), the server **hard-rejects the push**. The offline user's data remains safe locally, but they are prevented from corrupting the master server state.

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
- `role` (Text: 'admin' or 'user')
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
- `project_id` (UUID, Foreign Key → `projects.id`)
- `title` (String)
- `key_data` (Text)
- `created_at` (Timestamp)

### `file_locks`
Tracks explicit, granular file locks to prevent concurrent modifications on individual artifacts. Locks are tied to the immutable `artifact_id` rather than a mutable string path, ensuring locks remain valid even if the file is moved or renamed locally via `rig mv`. Furthermore, locks are **branch-isolated**, meaning a lock is granted for a specific artifact *on a specific branch*.
- `id` (UUID, Primary Key)
- `project_id` (UUID, Foreign Key → `projects.id`)
- `artifact_id` (String)
- `branch` (String)
- `locked_by` (String)
- `lock_generation_id` (UUID, Auto-generated token for push-time validation)
- `locked_at` (Timestamp)
- `updated_at` (Timestamp)
