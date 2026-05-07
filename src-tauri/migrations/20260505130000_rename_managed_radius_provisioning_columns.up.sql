DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'pppoe_accounts'
          AND column_name = 'radius_present'
    ) THEN
        ALTER TABLE public.pppoe_accounts
            RENAME COLUMN radius_present TO is_provisioned;
    END IF;

    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'pppoe_accounts'
          AND column_name = 'radius_last_sync_at'
    ) THEN
        ALTER TABLE public.pppoe_accounts
            RENAME COLUMN radius_last_sync_at TO provisioned_at;
    END IF;

    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'pppoe_accounts'
          AND column_name = 'radius_last_error'
    ) THEN
        ALTER TABLE public.pppoe_accounts
            RENAME COLUMN radius_last_error TO provisioning_error;
    END IF;
END $$;
