# RFC 0001: Local Disk Garbage Collection (Pruning)

- **Status**: Proposed
- **Created**: 2026-03-17

## 1. Abstract
Rig currently uses a **Lazy-Loading (0-byte Placeholder)** mechanism to save network bandwidth and local disk space. However, there is no defined mechanism to **revert** downloaded files back to their 0-byte placeholder state to reclaim disk space after working with large assets. This RFC proposes a `rig purge` command to handle local disk garbage collection.

## 2. Motivation
Once a user is finished working with a massive asset (e.g., they render a 50GB video, push it to the server, and no longer need the raw files locally), they cannot easily free up that 50GB of disk space without deleting the directory entirely and re-cloning. To truly support large-scale asset management, users must be able to gracefully "evict" local payloads.

## 3. Proposal
We propose introducing a command (`rig purge <path>`) that allows a user to evict the local payload of a tracked file.

**Expected Workflow:**
1. User runs `rig purge path/to/large_video.mp4`
2. Rig checks if the local file has any uncommitted/unpushed changes or is currently `locked`.
   - If YES: Abort with an error (preventing data loss).
   - If NO: Delete the physical file payload from `.rig/artifacts/` and replace the working directory file with a 0-byte placeholder.
3. The `.rig/index` is updated to reflect that the data payload is no longer held locally.
