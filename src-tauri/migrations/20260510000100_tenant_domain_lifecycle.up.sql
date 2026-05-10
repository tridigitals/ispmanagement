ALTER TABLE tenants
ADD COLUMN custom_domain_status text NOT NULL DEFAULT 'none';

ALTER TABLE tenants
ADD COLUMN custom_domain_verified_at timestamp with time zone;

ALTER TABLE tenants
ADD COLUMN custom_domain_failure_reason text;

UPDATE tenants
SET custom_domain_status = CASE
    WHEN custom_domain IS NOT NULL AND btrim(custom_domain) <> '' THEN 'active'
    ELSE 'none'
END;
