# rig

Binary to rig, code to git.

Rig is a tool designed to manage binary assets efficiently while being fully Git-aware. It allows you to keep Git inside your binary files for code, enabling seamless collaboration between artists and developers. With Rig, you can bridge the gap between creative and technical workflows, ensuring smooth version control and asset management in your projects.

## Commands

```
rig clone <url> [path]          # Clone a repository
rig status                      # Show working tree status
rig pull <path>                 # Pull specific artifact
rig fetch                       # Fetch latest metadata
rig log                         # Show commit history

rig lock <path>                 # Lock artifact for editing (makes writable)
rig add <path>                  # Add changes to artifact
rig commit -m "message"         # Create local commit
rig push                        # Push changes and commits to server

rig unlock <path>               # Unlock artifact (makes read-only)
```

## Workflow

1. **Clone**: `rig clone http://localhost:3000/MyProject`
2. **Edit**: `rig lock file.txt` (makes file writable)
3. **Modify**: Edit `file.txt`
4. **Stage**: `rig add file.txt` (uploads revision, sets read-only)
5. **Commit**: `rig commit -m "Updated file"`
6. **Push**: `rig push` (pushes revisions and commits)
7. **View History**: `rig log`
