DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'pppoe_accounts'
          AND column_name = 'is_provisioned'
    ) THEN
        ALTER TABLE public.pppoe_accounts
            RENAME COLUMN is_provisioned TO radius_present;
    END IF;

    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'pppoe_accounts'
          AND column_name = 'provisioned_at'
    ) THEN
        ALTER TABLE public.pppoe_accounts
            RENAME COLUMN provisioned_at TO radius_last_sync_at;
    END IF;

    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'pppoe_accounts'
          AND column_name = 'provisioning_error'
    ) THEN
        ALTER TABLE public.pppoe_accounts
            RENAME COLUMN provisioning_error TO radius_last_error;
    END IF;
END $$;
