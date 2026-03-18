-- Migration 005: Create auth tokens and CLI sessions
-- Fix: expires_at을 스펙(HTTP_AUTH.md) 기준 5분으로 수정. (기존 10분은 스펙 불일치)

CREATE TABLE tokens (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_text   TEXT NOT NULL UNIQUE,
    name         TEXT,
    created_at   TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMP WITH TIME ZONE
);

CREATE TABLE cli_sessions (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    status     TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'success', 'expired')),
    token_id   UUID REFERENCES tokens(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP WITH TIME ZONE DEFAULT (CURRENT_TIMESTAMP + INTERVAL '5 minutes')  -- spec: HTTP_AUTH.md
);