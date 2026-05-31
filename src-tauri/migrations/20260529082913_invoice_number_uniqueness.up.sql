-- HIGH #3 (MVP DoD audit): invoice number generation uniqueness.
--
-- Problem: PaymentService::create_invoice used `INV-{YYYYMMDD-HHMMSS}` which
-- collides at second granularity when the scheduler and a manual create
-- happen in the same second. The baseline schema only had a global UNIQUE
-- constraint on `invoice_number`, which (a) raises a hard conflict on burst
-- and (b) is wrong for multi-tenancy because two tenants must be able to
-- run independent numbering.
--
-- Fix:
--   1. Replace global UNIQUE with composite UNIQUE (tenant_id, invoice_number).
--   2. Provide a Postgres SEQUENCE used by the application to build a
--      collision-resistant number `INV-{YYYYMMDD}-{NNNNNN}`.
--
-- The application also retries on 23505 as a safety net for two writers
-- racing on the same nextval, but the structural guarantee here is the
-- combination of monotonic sequence + composite unique index.

-- ---------------------------------------------------------------------------
-- Safety: refuse to apply if existing data violates the new composite key.
-- This protects against silent corruption when migrating older deployments.
-- ---------------------------------------------------------------------------
DO $$
DECLARE
    dup_count int;
BEGIN
    SELECT COUNT(*) INTO dup_count FROM (
        SELECT tenant_id, invoice_number
        FROM invoices
        GROUP BY tenant_id, invoice_number
        HAVING COUNT(*) > 1
    ) d;
    IF dup_count > 0 THEN
        RAISE EXCEPTION
            'Found % duplicate (tenant_id, invoice_number) rows in invoices; resolve before applying constraint',
            dup_count;
    END IF;
END $$;

-- ---------------------------------------------------------------------------
-- Drop the old global UNIQUE on invoice_number (multi-tenancy concern).
-- Use IF EXISTS so the migration is idempotent and tolerates installs that
-- never had the baseline constraint.
-- ---------------------------------------------------------------------------
ALTER TABLE invoices DROP CONSTRAINT IF EXISTS invoices_invoice_number_key;

-- ---------------------------------------------------------------------------
-- Composite uniqueness scoped per-tenant.
-- ---------------------------------------------------------------------------
CREATE UNIQUE INDEX IF NOT EXISTS idx_invoices_tenant_invoice_number
    ON invoices (tenant_id, invoice_number);

-- ---------------------------------------------------------------------------
-- Monotonic sequence used by the application to build invoice numbers.
-- Decision: kept GLOBAL (not per-day), so values strictly increase across
-- days. This keeps the implementation deterministic and removes the need to
-- coordinate a daily reset with the scheduler. Acceptable for MVP; revisit
-- if/when invoice numbers need to be human-friendly per-day counters.
-- ---------------------------------------------------------------------------
CREATE SEQUENCE IF NOT EXISTS invoice_number_seq;
