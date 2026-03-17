# Rig Architecture Philosophy & Trust Model

The Rig version control system is built upon a strict, unyielding architectural philosophy. Every feature, database modification, and client command must be evaluated against these three core axioms:

### 1. The Core Guarantee: Zero Logical Errors in Concurrency
Rig exists to solve the fundamental problem of binary asset version control: **Binaries cannot be merged.** 
Therefore, concurrent modifications to the same file on the same branch must be structurally impossible. There are no "warnings," "soft limits," or "merge conflicts" for binary data. Any architectural flaw or edge case that could lead to two users successfully pushing conflicting changes to the same `artifact_id` on the same `branch` is considered a critical system failure. 

### 2. The Client is Fast, but Untrusted (UX over Security)
The Rig client (`rig` CLI) is designed for speed and offline flexibility. 
We explicitly acknowledge that the local file system is outside of Rig's absolute control. A user can trivially bypass local `.rig/` restrictions—they can manually change `r--` permissions to `rw-` using OS commands, edit files without running `rig lock`, hex-edit their local `.rig/index`, or delete tracking files. 

**The client's role is not security.** Client-side checks (such as blocking `rig unlock` when there are unpushed commits) exist purely as **User Experience (UX) guardrails** to prevent innocent mistakes and guide the user toward the correct workflow. We do not rely on the client to enforce the Core Guarantee.

### 3. The Server is the Fortress (Zero-Trust Security)
While the client is allowed to be messy, the server must be mathematically bulletproof. 
The Rig server treats every incoming network request (`pull`, `lock`, `unlock`, `push`) as potentially malicious or coming from a compromised, desynchronized client. 

To enforce the Core Guarantee, the server must implement **Zero-Trust Validation** for all state-mutating operations:
- **Push-Time Lock Validation:** The server does not assume the client still owns a lock just because they are pushing a file. When a client attempts to `rig push` a new revision for an `artifact_id`, the server **must independently verify** that the pushing user currently holds the exact `lock_generation_id` for that `artifact_id` on that specific `branch`. If the lock was hijacked, forced unlocked by an admin, or never formally acquired, the push is **hard-rejected**.
- **Immutable Artifacts:** Artifact blobs are written once and never modified. 
- **Decoupled Paths:** `artifact_id` and `branch` are the only trusted identifiers. String-based file paths (`assets/weapon.png`) are metadata strictly for the client's working directory UX and are never used by the server to validate locks or sequence revisions.
