-- Migration 003: Artifact metadata, revisions, and commit history

-- ============================================================
-- artifacts
-- 각 tracked 파일의 불변 메타데이터.
-- artifact_id는 클라이언트가 `rig add` 시 생성한 UUID v4를 그대로 사용.
-- path는 UX용 메타데이터일 뿐, 서버 검증에는 사용되지 않음.
-- ============================================================
CREATE TABLE IF NOT EXISTS artifacts (
    id             TEXT        PRIMARY KEY,  -- artifact_id (UUID v4, client-generated)
    project_id     UUID        NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    branch         TEXT        NOT NULL,
    path           TEXT        NOT NULL,     -- 논리적 경로 (UX 전용, 락/시퀀스 검증에 미사용)
    current_revision INTEGER   NOT NULL DEFAULT 0,
    created_at     TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at     TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    -- 동일 프로젝트+브랜치 내 path 충돌 방지 (Namespace Collision Check)
    -- 원자적 INSERT로 TOCTOU 없이 강제됨
    UNIQUE(project_id, branch, path)
);

-- ============================================================
-- artifact_revisions
-- push 성공 시 서버가 기록하는 불변 revision blob 메타데이터.
-- 실제 바이너리는 파일시스템의 artifacts/[artifact_id]/rev_N 에 저장.
-- ============================================================
CREATE TABLE IF NOT EXISTS artifact_revisions (
    id           UUID    PRIMARY KEY DEFAULT gen_random_uuid(),
    artifact_id  TEXT    NOT NULL REFERENCES artifacts(id) ON DELETE CASCADE,
    project_id   UUID    NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    revision     INTEGER NOT NULL,
    hash         TEXT    NOT NULL,  -- blake3 hash, push-time 서버 검증 완료본
    size         BIGINT  NOT NULL,
    author_id    UUID    NOT NULL REFERENCES users(id),
    commit_id    UUID,               -- 연결된 commits.id (nullable: 단독 push 허용)
    pushed_at    TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    UNIQUE(artifact_id, revision)   -- 동일 artifact의 revision 번호 중복 불가
);

-- ============================================================
-- commits
-- push 시 서버에 기록되는 커밋 단위.
-- 클라이언트의 로컬 .rig/objects/<uuid> 와 동일한 구조를 서버에서 영속화.
-- ============================================================
CREATE TABLE IF NOT EXISTS commits (
    id           UUID    PRIMARY KEY,           -- 클라이언트가 생성한 UUID (로컬 commit uuid)
    project_id   UUID    NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    branch       TEXT    NOT NULL,
    parent_id    UUID    REFERENCES commits(id), -- null이면 해당 브랜치의 첫 커밋
    author_id    UUID    NOT NULL REFERENCES users(id),
    message      TEXT    NOT NULL,
    committed_at TIMESTAMP WITH TIME ZONE NOT NULL  -- 클라이언트 로컬 timestamp (epoch → timestamptz)
);

-- ============================================================
-- commit_artifacts
-- 커밋 단위로 어떤 artifact가 어떤 op로 변경됐는지 기록.
-- revision_base는 push-time Stale Lineage 검증의 핵심 필드.
-- ============================================================
CREATE TABLE IF NOT EXISTS commit_artifacts (
    id             UUID    PRIMARY KEY DEFAULT gen_random_uuid(),
    commit_id      UUID    NOT NULL REFERENCES commits(id) ON DELETE CASCADE,
    artifact_id    TEXT    NOT NULL REFERENCES artifacts(id),
    path           TEXT    NOT NULL,    -- 커밋 시점의 논리 경로 스냅샷
    revision_base  INTEGER NOT NULL,    -- 클라이언트가 편집 시작한 서버 revision
    hash           TEXT,                -- blake3 hash. DELETE op이면 NULL
    op             TEXT    NOT NULL CHECK (op IN ('upsert', 'delete'))
);