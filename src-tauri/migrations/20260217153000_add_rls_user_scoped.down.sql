-- Rollback user-scoped RLS policies

DROP POLICY IF EXISTS p_users_self ON public.users;
ALTER TABLE public.users DISABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS p_sessions_owner ON public.sessions;
ALTER TABLE public.sessions DISABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS p_trusted_devices_owner ON public.trusted_devices;
ALTER TABLE public.trusted_devices DISABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS p_user_addresses_owner ON public.user_addresses;
ALTER TABLE public.user_addresses DISABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS p_oauth_accounts_owner ON public.oauth_accounts;
ALTER TABLE public.oauth_accounts DISABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS p_push_subscriptions_owner ON public.push_subscriptions;
ALTER TABLE public.push_subscriptions DISABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS p_notification_preferences_owner ON public.notification_preferences;
ALTER TABLE public.notification_preferences DISABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS p_notifications_owner ON public.notifications;
ALTER TABLE public.notifications DISABLE ROW LEVEL SECURITY;
