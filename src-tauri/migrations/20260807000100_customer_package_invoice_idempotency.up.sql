-- Prevent concurrent bootstrap repair from creating duplicate customer-package invoices.
-- Only managed subscription invoices are covered; manual/external invoice IDs remain unaffected.
DO $$
DECLARE
    dup_count int;
BEGIN
    SELECT COUNT(*) INTO dup_count
    FROM (
        SELECT tenant_id, external_id
        FROM invoices
        WHERE external_id LIKE 'pkgsub:%'
        GROUP BY tenant_id, external_id
        HAVING COUNT(*) > 1
    ) duplicates;

    IF dup_count > 0 THEN
        RAISE EXCEPTION
            'Found % duplicate (tenant_id, external_id) customer-package invoice groups; resolve before applying constraint',
            dup_count;
    END IF;
END $$;

CREATE UNIQUE INDEX IF NOT EXISTS idx_invoices_tenant_customer_package_external_id
    ON invoices (tenant_id, external_id)
    WHERE external_id LIKE 'pkgsub:%';
