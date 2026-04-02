DROP TABLE IF EXISTS public.managed_radius_nas;
DROP TABLE IF EXISTS public.managed_radius_servers;

DROP INDEX IF EXISTS public.idx_pppoe_accounts_tenant_source;

ALTER TABLE public.pppoe_accounts
    DROP CONSTRAINT IF EXISTS chk_pppoe_accounts_account_source;

ALTER TABLE public.pppoe_accounts
    DROP COLUMN IF EXISTS radius_last_error,
    DROP COLUMN IF EXISTS radius_last_sync_at,
    DROP COLUMN IF EXISTS radius_identity,
    DROP COLUMN IF EXISTS radius_present,
    DROP COLUMN IF EXISTS account_source;
