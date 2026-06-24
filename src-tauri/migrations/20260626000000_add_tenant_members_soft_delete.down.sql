-- Remove soft-delete column from tenant_members
ALTER TABLE tenant_members DROP COLUMN IF EXISTS deleted_at;
