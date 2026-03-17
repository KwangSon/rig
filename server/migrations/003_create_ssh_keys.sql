CREATE TABLE IF NOT EXISTS ssh_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project VARCHAR(255) NOT NULL REFERENCES projects(name) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    key_data TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(project, key_data)
);
