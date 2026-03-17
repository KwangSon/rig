-- Create file_locks table
CREATE TABLE IF NOT EXISTS file_locks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    artifact_id TEXT NOT NULL,
    branch TEXT NOT NULL,
    locked_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    lock_generation_id UUID NOT NULL DEFAULT gen_random_uuid(),
    locked_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(project_id, artifact_id, branch)
);
