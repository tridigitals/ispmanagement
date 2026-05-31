-- Down migration for invoice_number_uniqueness.
--
-- Best-effort revert. Notes:
--
-- 1. The composite unique index is dropped first because it is the structural
--    constraint that protects multi-tenant numbering. Dropping it returns the
--    schema to a state where collisions can occur silently.
--
-- 2. The previous global UNIQUE on `invoice_number` is restored ONLY when the
--    current data still satisfies it. If multiple tenants emitted overlapping
--    numbers under the new scheme, the constraint cannot be reinstated;
--    operators must resolve the data first. We do this conditionally rather
--    than failing the down migration outright so partial rollback remains
--    possible.
--
-- 3. The sequence `invoice_number_seq` is dropped. Any code path that still
--    references it after a rollback will fail loudly, which is preferable to
--    silently regressing to colliding HHMMSS numbers.

DROP INDEX IF EXISTS idx_invoices_tenant_invoice_number;

DO $$
DECLARE
    dup_count int;
BEGIN
    SELECT COUNT(*) INTO dup_count FROM (
        SELECT invoice_number
        FROM invoices
        GROUP BY invoice_number
        HAVING COUNT(*) > 1
    ) d;
    IF dup_count = 0 THEN
        BEGIN
            ALTER TABLE invoices
                ADD CONSTRAINT invoices_invoice_number_key UNIQUE (invoice_number);
        EXCEPTION WHEN duplicate_table OR duplicate_object THEN
            -- Constraint already present; nothing to do.
            NULL;
        END;
    ELSE
        RAISE NOTICE
            'Skipping restore of global UNIQUE (invoice_number): % cross-tenant duplicates exist.',
            dup_count;
    END IF;
END $$;

DROP SEQUENCE IF EXISTS invoice_number_seq;
