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

## Server-Side Storage Structure

On the server, data is partitioned and isolated by `project_id`. This multi-tenant design ensures that all files and metadata are strictly scoped to their respective projects. 

The internal storage layout for a specific project looks like this:

```text
[project_id]/
 ├── .rig/         # Core metadata, configuration, index, and commit history
 └── artifacts/    # The physical object store for actual file data
```

### The `.rig/` Directory
This directory serves the exact same purpose as it does on the client. It contains all the structural metadata for the project, including:
- `index.json`: Maps logical file paths to their artifact hashes, sizes, current revisions, and locking states.
- `config.json`: Project-specific configuration containing remote server mapping (if applicable) and branch schemas.
- **Commit/Revision Graph**: Maintains the global history pointer of what changed and when.

### The `artifacts/` Directory
This directory acts as the object store mechanism for the project. While the `.rig/` directory knows *about* the files, the `artifacts/` directory holds the concrete file data (blobs). 
- Files stored here are usually named or hashed by their artifact ID to decouple the physical storage from the logical file path. 
- These are the "real" file payloads that are transferred to users when they perform a `pull` request.

## DB schema

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
- `project` (Text)
- `access` (Text: 'read', 'write', or 'admin')

### `ssh_keys`
Stores public SSH keys for secure artifact and git submodule access management.
- `id` (UUID, Primary Key)
- `project` (String, Foreign Key → `projects.name`)
- `title` (String)
- `key_data` (Text)
- `created_at` (Timestamp)

### `file_locks`
Tracks explicit, granular file locks to prevent concurrent modifications on individual artifacts.
- `id` (UUID, Primary Key)
- `project` (String, Foreign Key → `projects.name`)
- `file_path` (String)
- `locked_by` (String)
- `locked_at` (Timestamp)
- `updated_at` (Timestamp)
