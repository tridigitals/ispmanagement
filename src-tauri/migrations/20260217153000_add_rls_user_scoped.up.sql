-- RLS user-scoped policies (staged, backward-compatible).
--
-- Uses app.current_user_id transaction-local context.
-- Fallback behavior remains permissive when current_user_id is not set yet.

-- users
ALTER TABLE public.users ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS p_users_self ON public.users;
CREATE POLICY p_users_self ON public.users
FOR ALL
USING (
    current_setting('app.current_is_superadmin', true) = 'true'
    OR nullif(current_setting('app.current_user_id', true), '') IS NULL
    OR id::text = nullif(current_setting('app.current_user_id', true), '')
)
WITH CHECK (
    current_setting('app.current_is_superadmin', true) = 'true'
    OR nullif(current_setting('app.current_user_id', true), '') IS NULL
    OR id::text = nullif(current_setting('app.current_user_id', true), '')
);

-- sessions
ALTER TABLE public.sessions ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS p_sessions_owner ON public.sessions;
CREATE POLICY p_sessions_owner ON public.sessions
FOR ALL
USING (
    current_setting('app.current_is_superadmin', true) = 'true'
    OR nullif(current_setting('app.current_user_id', true), '') IS NULL
    OR user_id::text = nullif(current_setting('app.current_user_id', true), '')
)
WITH CHECK (
    current_setting('app.current_is_superadmin', true) = 'true'
    OR nullif(current_setting('app.current_user_id', true), '') IS NULL
    OR user_id::text = nullif(current_setting('app.current_user_id', true), '')
);

-- trusted_devices
ALTER TABLE public.trusted_devices ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS p_trusted_devices_owner ON public.trusted_devices;
CREATE POLICY p_trusted_devices_owner ON public.trusted_devices
FOR ALL
USING (
    current_setting('app.current_is_superadmin', true) = 'true'
    OR nullif(current_setting('app.current_user_id', true), '') IS NULL
    OR user_id::text = nullif(current_setting('app.current_user_id', true), '')
)
WITH CHECK (
    current_setting('app.current_is_superadmin', true) = 'true'
    OR nullif(current_setting('app.current_user_id', true), '') IS NULL
    OR user_id::text = nullif(current_setting('app.current_user_id', true), '')
);

-- user_addresses
ALTER TABLE public.user_addresses ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS p_user_addresses_owner ON public.user_addresses;
CREATE POLICY p_user_addresses_owner ON public.user_addresses
FOR ALL
USING (
    current_setting('app.current_is_superadmin', true) = 'true'
    OR nullif(current_setting('app.current_user_id', true), '') IS NULL
    OR user_id::text = nullif(current_setting('app.current_user_id', true), '')
)
WITH CHECK (
    current_setting('app.current_is_superadmin', true) = 'true'
    OR nullif(current_setting('app.current_user_id', true), '') IS NULL
    OR user_id::text = nullif(current_setting('app.current_user_id', true), '')
);

-- oauth_accounts
ALTER TABLE public.oauth_accounts ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS p_oauth_accounts_owner ON public.oauth_accounts;
CREATE POLICY p_oauth_accounts_owner ON public.oauth_accounts
FOR ALL
USING (
    current_setting('app.current_is_superadmin', true) = 'true'
    OR nullif(current_setting('app.current_user_id', true), '') IS NULL
    OR user_id::text = nullif(current_setting('app.current_user_id', true), '')
)
WITH CHECK (
    current_setting('app.current_is_superadmin', true) = 'true'
    OR nullif(current_setting('app.current_user_id', true), '') IS NULL
    OR user_id::text = nullif(current_setting('app.current_user_id', true), '')
);

-- push_subscriptions
ALTER TABLE public.push_subscriptions ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS p_push_subscriptions_owner ON public.push_subscriptions;
CREATE POLICY p_push_subscriptions_owner ON public.push_subscriptions
FOR ALL
USING (
    current_setting('app.current_is_superadmin', true) = 'true'
    OR nullif(current_setting('app.current_user_id', true), '') IS NULL
    OR user_id::text = nullif(current_setting('app.current_user_id', true), '')
)
WITH CHECK (
    current_setting('app.current_is_superadmin', true) = 'true'
    OR nullif(current_setting('app.current_user_id', true), '') IS NULL
    OR user_id::text = nullif(current_setting('app.current_user_id', true), '')
);

-- notification_preferences
ALTER TABLE public.notification_preferences ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS p_notification_preferences_owner ON public.notification_preferences;
CREATE POLICY p_notification_preferences_owner ON public.notification_preferences
FOR ALL
USING (
    current_setting('app.current_is_superadmin', true) = 'true'
    OR nullif(current_setting('app.current_user_id', true), '') IS NULL
    OR user_id::text = nullif(current_setting('app.current_user_id', true), '')
)
WITH CHECK (
    current_setting('app.current_is_superadmin', true) = 'true'
    OR nullif(current_setting('app.current_user_id', true), '') IS NULL
    OR user_id::text = nullif(current_setting('app.current_user_id', true), '')
);

-- notifications
ALTER TABLE public.notifications ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS p_notifications_owner ON public.notifications;
CREATE POLICY p_notifications_owner ON public.notifications
FOR ALL
USING (
    current_setting('app.current_is_superadmin', true) = 'true'
    OR nullif(current_setting('app.current_user_id', true), '') IS NULL
    OR user_id::text = nullif(current_setting('app.current_user_id', true), '')
    OR user_id IS NULL
)
WITH CHECK (
    current_setting('app.current_is_superadmin', true) = 'true'
    OR nullif(current_setting('app.current_user_id', true), '') IS NULL
    OR user_id::text = nullif(current_setting('app.current_user_id', true), '')
    OR user_id IS NULL
);
