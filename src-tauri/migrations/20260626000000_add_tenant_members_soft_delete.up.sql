-- Add soft-delete column to tenant_members
ALTER TABLE tenant_members ADD COLUMN deleted_at TIMESTAMPTZ;

-- Add index for filtering active members
CREATE INDEX IF NOT EXISTS idx_tenant_members_deleted_at ON tenant_members(deleted_at);
