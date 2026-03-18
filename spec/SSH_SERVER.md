# Rig SSH Server Specification

The Rig SSH Server provides a secure, high-performance transport layer for binary asset management, complementing the existing HTTP API.

## Goals

1.  **Secure Authentication**: Use SSH Public Key Authentication (standard and cryptographically robust).
2.  **High-Performance Streaming**: Leverages SSH channels for raw binary transfer, reducing overhead compared to standard HTTP/JSON for massive assets.
3.  **Command Execution Environment**: Mimics the Git SSH model (e.g., `git-receive-pack`) to route specific Rig commands.
4.  **Symmetry**: Uses the same underlying data store and database as the HTTP server.

## Architecture

### 1. Transport Layer
- **Port**: Default 2022 (to avoid conflict with system SSH on 22).
- **Library**: Built using `russh` or `async-ssh2` in the Rig server.
- **Protocol**: Standard SSHv2.

### 2. Authentication Flow
- The server maintains a `ssh_keys` table.
- When a client connects, the SSH server performs a lookup for the provided public key.
- If a match is found, the session is associated with the corresponding `user_id`.

### 3. Command Routing
Rig users will use SSH URLs for remote operations:
`ssh://localhost:2022/username/project`

When a command is executed over SSH (e.g., `ssh user@host rig-receive-pack project`), the server:
1.  Verifies the user's permission for the project.
2.  Executes the internal handler for that command.
3.  Streams stdin/stdout directly through the SSH channel.

## Communication Protocol (Pipe Pattern)

Rig over SSH uses a simple packet-based protocol within the SSH channel:

1.  **Handshake**: Client sends version and capabilities.
2.  **Command Request**: Client sends the command (e.g., `push`, `pull`, `lock`).
3.  **Data Stream**: 
    - For `push`: Client streams a series of (metadata + chunk data) packets.
    - For `pull`: Server streams back the requested blobs.
4.  **Termination**: Final status code and message.

## Security Considerations

- **Strict Command Whitelisting**: The SSH server *only* allows execution of `rig-*` commands. No shell access is provided.
- **Authorization Enforcement**: Every command request is verified against the `permissions` table.
- **Rate Limiting**: SSH connections are rate-limited per user/IP.

## Future Plans: Rig Gateway

The SSH server can eventually act as a gateway/proxy, routing requests to different storage nodes based on the `project_id`.
