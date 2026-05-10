ALTER TABLE tenants
DROP COLUMN IF EXISTS custom_domain_failure_reason;

ALTER TABLE tenants
DROP COLUMN IF EXISTS custom_domain_verified_at;

ALTER TABLE tenants
DROP COLUMN IF EXISTS custom_domain_status;
