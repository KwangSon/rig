# HTTP Token Authentication Specification

## Overview
Rig uses a token-based authentication system over HTTP. The system supports Just-In-Time (JIT) authentication for CLI commands via a browser-based login flow.

## Authentication Flow (CLI)
1. **Initiation**: A CLI command (e.g., `rig clone`) detects that authentication is required.
2. **Session Creation**: The client calls `POST /api/v1/auth/session` to create a `cli_session`.
3. **User Interaction**: The client opens the user's browser to `http://localhost:3002/auth/login?cli_session=<id>`.
4. **Login**: The user logs in via the Web UI.
5. **Approval**: Upon successful login, the server associates the authenticated `user_id` with the `cli_session`.
6. **Polling**: The client polls `GET /api/v1/auth/token?session_id=<id>` until a token is returned.
7. **Storage**: The client stores the token in `~/.rig/credentials` (formatted as JSON, file permissions `600`).
8. **Execution**: The client uses the token in the `Authorization: Bearer <token>` header for all subsequent API calls.

## Credential Storage
Location: `~/.rig/credentials`
Permissions: `0600` (User Read/Write ONLY)
Format:
```json
{
  "host_tokens": {
    "http://localhost:3000": "your-token-here"
  }
}
```

## Security Considerations
- Tokens are currently non-expiring for testing purposes.
- CLI sessions expire after 5 minutes if not approved.
- All sensitive API endpoints are protected by `authenticate_token` middleware.
