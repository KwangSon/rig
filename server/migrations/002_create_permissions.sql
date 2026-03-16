-- Create permissions table
CREATE TABLE IF NOT EXISTS permissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    project TEXT NOT NULL,
    access TEXT NOT NULL CHECK (access IN ('read', 'write', 'admin')),
    UNIQUE(user_id, project)
);

-- Insert fixture permissions
INSERT INTO permissions (user_id, project, access) VALUES
('550e8400-e29b-41d4-a716-446655440000'::uuid, 'ExampleProject', 'admin'),
('550e8400-e29b-41d4-a716-446655440001'::uuid, 'ExampleProject', 'read')
ON CONFLICT (user_id, project) DO NOTHING;