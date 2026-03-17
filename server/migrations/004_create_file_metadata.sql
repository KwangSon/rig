CREATE TABLE IF NOT EXISTS file_locks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project VARCHAR(255) NOT NULL REFERENCES projects(name) ON DELETE CASCADE,
    file_path VARCHAR(512) NOT NULL,
    locked_by VARCHAR(255),
    locked_at TIMESTAMP WITH TIME ZONE,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(project, file_path)
);
