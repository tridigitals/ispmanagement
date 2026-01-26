-- Add 2FA columns to users table
ALTER TABLE users ADD COLUMN IF NOT EXISTS two_factor_enabled BOOLEAN DEFAULT FALSE;
ALTER TABLE users ADD COLUMN IF NOT EXISTS two_factor_secret TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS two_factor_recovery_codes TEXT;

-- Add enforcement flag to tenants table
ALTER TABLE tenants ADD COLUMN IF NOT EXISTS enforce_2fa BOOLEAN DEFAULT FALSE;
