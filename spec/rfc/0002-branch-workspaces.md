# RFC 0002: Branch-Level Isolated Workspaces

- **Status**: Draft
- **Created**: 2026-03-17

## 1. Abstract
When running `rig checkout <branch>`, Rig currently replaces the files in the working directory to match the target branch (similar to Git). However, for large binary projects (e.g., Unreal Engine or Unity), swapping out 100GB of assets in place can take an extremely long time and thrash the disk. This RFC proposes "Branch-Level Isolated Workspaces" where branches can be mounted or downloaded into separate, parallel directories.

## 2. Motivation
In software engineering, checking out a branch is fast because text files are small. In 3D art or game development, switching branches might mean replacing gigabytes of textures and models. If a user needs to quickly check a different branch and switch back, the disk I/O cost is prohibitively high. 

## 3. Proposal
Introduce an option to clone or checkout branches into dedicated sub-directories or entirely separate workspace roots.
- Example: `rig worktree add experiment-branch ./experiment-workspace`
- This allows the user to have two branches open on disk simultaneously without needing to re-download or overwrite massive assets.
