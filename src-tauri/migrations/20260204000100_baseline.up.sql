-- Baseline schema (source of truth for Postgres)
-- Generated from current dev DB schema; do not edit after it has been applied.

CREATE TABLE public.audit_logs (
    id uuid NOT NULL,
    user_id uuid,
    tenant_id uuid,
    action character varying(255) NOT NULL,
    resource character varying(255) NOT NULL,
    resource_id text,
    details text,
    ip_address character varying(45),
    created_at timestamp with time zone NOT NULL
);


-- Name: bank_accounts; Type: TABLE; Schema: public; Owner: -

CREATE TABLE public.bank_accounts (
    id text NOT NULL,
    bank_name text NOT NULL,
    account_number text NOT NULL,
    account_holder text NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL
);


-- Name: features; Type: TABLE; Schema: public; Owner: -

CREATE TABLE public.features (
    id text NOT NULL,
    code text NOT NULL,
    name text NOT NULL,
    description text,
    value_type text DEFAULT 'boolean'::text NOT NULL,
    category text DEFAULT 'general'::text,
    default_value text DEFAULT 'false'::text,
    sort_order integer DEFAULT 0,
    created_at timestamp with time zone NOT NULL
);


-- Name: file_records; Type: TABLE; Schema: public; Owner: -

CREATE TABLE public.file_records (
    id text NOT NULL,
    tenant_id text NOT NULL,
    name text NOT NULL,
    original_name text NOT NULL,
    path text NOT NULL,
    size bigint NOT NULL,
    content_type text NOT NULL,
    uploaded_by text,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    storage_provider text DEFAULT 'local'::text NOT NULL
);


-- Name: fx_rates; Type: TABLE; Schema: public; Owner: -

CREATE TABLE public.fx_rates (
    base_currency text NOT NULL,
    quote_currency text NOT NULL,
    rate numeric(18,8) NOT NULL,
    fetched_at timestamp with time zone NOT NULL,
    source text NOT NULL
);


-- Name: invoices; Type: TABLE; Schema: public; Owner: -

CREATE TABLE public.invoices (
    id text NOT NULL,
    tenant_id text NOT NULL,
    invoice_number text NOT NULL,
    amount numeric(10,2) NOT NULL,
    status text DEFAULT 'pending'::text NOT NULL,
    description text,
    due_date timestamp with time zone NOT NULL,
    paid_at timestamp with time zone,
    payment_method text,
    external_id text,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    merchant_id text,
    proof_attachment text,
    currency_code text DEFAULT 'IDR'::text NOT NULL,
    base_currency_code text DEFAULT 'IDR'::text NOT NULL,
    fx_rate numeric(18,8),
    fx_source text,
    fx_fetched_at timestamp with time zone
);


-- Name: notification_preferences; Type: TABLE; Schema: public; Owner: -

CREATE TABLE public.notification_preferences (
    id text NOT NULL,
    user_id text NOT NULL,
    channel text NOT NULL,
    category text NOT NULL,
    enabled boolean DEFAULT true NOT NULL,
    updated_at timestamp with time zone NOT NULL
);


-- Name: notifications; Type: TABLE; Schema: public; Owner: -

CREATE TABLE public.notifications (
    id text NOT NULL,
    user_id text,
    tenant_id text,
    title text NOT NULL,
    message text NOT NULL,
    type text DEFAULT 'info'::text NOT NULL,
    is_read boolean DEFAULT false NOT NULL,
    link text,
    created_at timestamp with time zone NOT NULL,
    notification_type text DEFAULT 'info'::text,
    category text DEFAULT 'system'::text,
    action_url text
);


-- Name: oauth_accounts; Type: TABLE; Schema: public; Owner: -

CREATE TABLE public.oauth_accounts (
    id text NOT NULL,
    user_id text NOT NULL,
    provider text NOT NULL,
    provider_user_id text NOT NULL,
    provider_email text,
    access_token text,
    refresh_token text,
    expires_at timestamp with time zone,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL
);


-- Name: permissions; Type: TABLE; Schema: public; Owner: -

CREATE TABLE public.permissions (
    id text NOT NULL,
    resource text NOT NULL,
    action text NOT NULL,
    description text
);


-- Name: plan_features; Type: TABLE; Schema: public; Owner: -

CREATE TABLE public.plan_features (
    id text NOT NULL,
    plan_id text NOT NULL,
    feature_id text NOT NULL,
    value text NOT NULL
);


-- Name: plans; Type: TABLE; Schema: public; Owner: -

CREATE TABLE public.plans (
    id text NOT NULL,
    name text NOT NULL,
    slug text NOT NULL,
    description text,
    price_monthly numeric(10,2) DEFAULT 0,
    price_yearly numeric(10,2) DEFAULT 0,
    is_active boolean DEFAULT true,
    is_default boolean DEFAULT false,
    sort_order integer DEFAULT 0,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL
);


-- Name: push_subscriptions; Type: TABLE; Schema: public; Owner: -

CREATE TABLE public.push_subscriptions (
    id text NOT NULL,
    user_id text NOT NULL,
    endpoint text NOT NULL,
    p256dh text NOT NULL,
    auth text NOT NULL,
    created_at timestamp with time zone NOT NULL
);


-- Name: role_permissions; Type: TABLE; Schema: public; Owner: -

CREATE TABLE public.role_permissions (
    role_id text NOT NULL,
    permission_id text NOT NULL
);


-- Name: roles; Type: TABLE; Schema: public; Owner: -

CREATE TABLE public.roles (
    id text NOT NULL,
    tenant_id text,
    name text NOT NULL,
    description text,
    is_system boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    level integer DEFAULT 0 NOT NULL
);


-- Name: sessions; Type: TABLE; Schema: public; Owner: -

CREATE TABLE public.sessions (
    id text NOT NULL,
    user_id text NOT NULL,
    tenant_id text,
    token text NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone NOT NULL
);


-- Name: settings; Type: TABLE; Schema: public; Owner: -

CREATE TABLE public.settings (
    id text NOT NULL,
    tenant_id text,
    key text NOT NULL,
    value text NOT NULL,
    description text,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL
);


-- Name: tenant_members; Type: TABLE; Schema: public; Owner: -

CREATE TABLE public.tenant_members (
    id text NOT NULL,
    tenant_id text NOT NULL,
    user_id text NOT NULL,
    role text DEFAULT 'member'::text NOT NULL,
    created_at timestamp with time zone NOT NULL,
    role_id text
);


-- Name: tenant_subscriptions; Type: TABLE; Schema: public; Owner: -

CREATE TABLE public.tenant_subscriptions (
    id text NOT NULL,
    tenant_id text NOT NULL,
    plan_id text NOT NULL,
    status text DEFAULT 'active'::text,
    trial_ends_at timestamp with time zone,
    current_period_start timestamp with time zone DEFAULT now(),
    current_period_end timestamp with time zone,
    feature_overrides jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL
);


-- Name: tenants; Type: TABLE; Schema: public; Owner: -

CREATE TABLE public.tenants (
    id text NOT NULL,
    name text NOT NULL,
    slug text NOT NULL,
    custom_domain text,
    logo_url text,
    is_active boolean DEFAULT true NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    storage_usage bigint DEFAULT 0 NOT NULL,
    enforce_2fa boolean DEFAULT false NOT NULL
);


-- Name: trusted_devices; Type: TABLE; Schema: public; Owner: -

CREATE TABLE public.trusted_devices (
    id text NOT NULL,
    user_id text NOT NULL,
    device_fingerprint character varying(255) NOT NULL,
    ip_address character varying(45),
    user_agent text,
    trusted_at timestamp with time zone DEFAULT now(),
    expires_at timestamp with time zone NOT NULL,
    last_used_at timestamp with time zone DEFAULT now()
);


-- Name: users; Type: TABLE; Schema: public; Owner: -

CREATE TABLE public.users (
    id text NOT NULL,
    email text NOT NULL,
    password_hash text NOT NULL,
    name text NOT NULL,
    role text DEFAULT 'user'::text NOT NULL,
    is_super_admin boolean DEFAULT false NOT NULL,
    avatar_url text,
    is_active boolean DEFAULT true NOT NULL,
    email_verified_at timestamp with time zone,
    failed_login_attempts integer DEFAULT 0 NOT NULL,
    locked_until timestamp with time zone,
    verification_token text,
    reset_token text,
    reset_token_expires timestamp with time zone,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    two_factor_enabled boolean DEFAULT false NOT NULL,
    two_factor_secret text,
    two_factor_recovery_codes text,
    email_otp_code text,
    email_otp_expires timestamp with time zone,
    preferred_2fa_method text DEFAULT 'totp'::text,
    totp_enabled boolean DEFAULT false NOT NULL,
    email_2fa_enabled boolean DEFAULT false NOT NULL
);


-- Name: audit_logs audit_logs_pkey; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.audit_logs
    ADD CONSTRAINT audit_logs_pkey PRIMARY KEY (id);


-- Name: bank_accounts bank_accounts_pkey; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.bank_accounts
    ADD CONSTRAINT bank_accounts_pkey PRIMARY KEY (id);


-- Name: features feature_definitions_code_key; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.features
    ADD CONSTRAINT feature_definitions_code_key UNIQUE (code);


-- Name: features feature_definitions_pkey; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.features
    ADD CONSTRAINT feature_definitions_pkey PRIMARY KEY (id);


-- Name: file_records file_records_pkey; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.file_records
    ADD CONSTRAINT file_records_pkey PRIMARY KEY (id);


-- Name: fx_rates fx_rates_pkey; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.fx_rates
    ADD CONSTRAINT fx_rates_pkey PRIMARY KEY (base_currency, quote_currency);


-- Name: invoices invoices_invoice_number_key; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.invoices
    ADD CONSTRAINT invoices_invoice_number_key UNIQUE (invoice_number);


-- Name: invoices invoices_pkey; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.invoices
    ADD CONSTRAINT invoices_pkey PRIMARY KEY (id);


-- Name: notification_preferences notification_preferences_pkey; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.notification_preferences
    ADD CONSTRAINT notification_preferences_pkey PRIMARY KEY (id);


-- Name: notification_preferences notification_preferences_user_id_channel_category_key; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.notification_preferences
    ADD CONSTRAINT notification_preferences_user_id_channel_category_key UNIQUE (user_id, channel, category);


-- Name: notifications notifications_pkey; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.notifications
    ADD CONSTRAINT notifications_pkey PRIMARY KEY (id);


-- Name: oauth_accounts oauth_accounts_pkey; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.oauth_accounts
    ADD CONSTRAINT oauth_accounts_pkey PRIMARY KEY (id);


-- Name: oauth_accounts oauth_accounts_provider_provider_user_id_key; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.oauth_accounts
    ADD CONSTRAINT oauth_accounts_provider_provider_user_id_key UNIQUE (provider, provider_user_id);


-- Name: permissions permissions_pkey; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.permissions
    ADD CONSTRAINT permissions_pkey PRIMARY KEY (id);


-- Name: permissions permissions_resource_action_key; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.permissions
    ADD CONSTRAINT permissions_resource_action_key UNIQUE (resource, action);


-- Name: plan_features plan_features_pkey; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.plan_features
    ADD CONSTRAINT plan_features_pkey PRIMARY KEY (id);


-- Name: plan_features plan_features_plan_id_feature_id_key; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.plan_features
    ADD CONSTRAINT plan_features_plan_id_feature_id_key UNIQUE (plan_id, feature_id);


-- Name: plans plans_pkey; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.plans
    ADD CONSTRAINT plans_pkey PRIMARY KEY (id);


-- Name: plans plans_slug_key; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.plans
    ADD CONSTRAINT plans_slug_key UNIQUE (slug);


-- Name: push_subscriptions push_subscriptions_endpoint_key; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.push_subscriptions
    ADD CONSTRAINT push_subscriptions_endpoint_key UNIQUE (endpoint);


-- Name: push_subscriptions push_subscriptions_pkey; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.push_subscriptions
    ADD CONSTRAINT push_subscriptions_pkey PRIMARY KEY (id);


-- Name: role_permissions role_permissions_pkey; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.role_permissions
    ADD CONSTRAINT role_permissions_pkey PRIMARY KEY (role_id, permission_id);


-- Name: roles roles_pkey; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.roles
    ADD CONSTRAINT roles_pkey PRIMARY KEY (id);


-- Name: sessions sessions_pkey; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.sessions
    ADD CONSTRAINT sessions_pkey PRIMARY KEY (id);


-- Name: sessions sessions_token_key; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.sessions
    ADD CONSTRAINT sessions_token_key UNIQUE (token);


-- Name: settings settings_pkey; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.settings
    ADD CONSTRAINT settings_pkey PRIMARY KEY (id);


-- Name: tenant_members tenant_members_pkey; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.tenant_members
    ADD CONSTRAINT tenant_members_pkey PRIMARY KEY (id);


-- Name: tenant_members tenant_members_tenant_id_user_id_key; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.tenant_members
    ADD CONSTRAINT tenant_members_tenant_id_user_id_key UNIQUE (tenant_id, user_id);


-- Name: tenant_subscriptions tenant_subscriptions_pkey; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.tenant_subscriptions
    ADD CONSTRAINT tenant_subscriptions_pkey PRIMARY KEY (id);


-- Name: tenant_subscriptions tenant_subscriptions_tenant_id_key; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.tenant_subscriptions
    ADD CONSTRAINT tenant_subscriptions_tenant_id_key UNIQUE (tenant_id);


-- Name: tenants tenants_custom_domain_key; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.tenants
    ADD CONSTRAINT tenants_custom_domain_key UNIQUE (custom_domain);


-- Name: tenants tenants_pkey; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.tenants
    ADD CONSTRAINT tenants_pkey PRIMARY KEY (id);


-- Name: tenants tenants_slug_key; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.tenants
    ADD CONSTRAINT tenants_slug_key UNIQUE (slug);


-- Name: trusted_devices trusted_devices_pkey; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.trusted_devices
    ADD CONSTRAINT trusted_devices_pkey PRIMARY KEY (id);


-- Name: trusted_devices trusted_devices_user_id_device_fingerprint_key; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.trusted_devices
    ADD CONSTRAINT trusted_devices_user_id_device_fingerprint_key UNIQUE (user_id, device_fingerprint);


-- Name: users users_email_key; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_email_key UNIQUE (email);


-- Name: users users_pkey; Type: CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_pkey PRIMARY KEY (id);


-- Name: idx_audit_logs_action; Type: INDEX; Schema: public; Owner: -

CREATE INDEX idx_audit_logs_action ON public.audit_logs USING btree (action);


-- Name: idx_audit_logs_created; Type: INDEX; Schema: public; Owner: -

CREATE INDEX idx_audit_logs_created ON public.audit_logs USING btree (created_at DESC);


-- Name: idx_audit_logs_tenant; Type: INDEX; Schema: public; Owner: -

CREATE INDEX idx_audit_logs_tenant ON public.audit_logs USING btree (tenant_id);


-- Name: idx_audit_logs_user; Type: INDEX; Schema: public; Owner: -

CREATE INDEX idx_audit_logs_user ON public.audit_logs USING btree (user_id);


-- Name: idx_feature_definitions_code; Type: INDEX; Schema: public; Owner: -

CREATE INDEX idx_feature_definitions_code ON public.features USING btree (code);


-- Name: idx_file_records_created; Type: INDEX; Schema: public; Owner: -

CREATE INDEX idx_file_records_created ON public.file_records USING btree (created_at DESC);


-- Name: idx_file_records_tenant; Type: INDEX; Schema: public; Owner: -

CREATE INDEX idx_file_records_tenant ON public.file_records USING btree (tenant_id);


-- Name: idx_invoices_created; Type: INDEX; Schema: public; Owner: -

CREATE INDEX idx_invoices_created ON public.invoices USING btree (created_at DESC);


-- Name: idx_invoices_status; Type: INDEX; Schema: public; Owner: -

CREATE INDEX idx_invoices_status ON public.invoices USING btree (status);


-- Name: idx_invoices_tenant; Type: INDEX; Schema: public; Owner: -

CREATE INDEX idx_invoices_tenant ON public.invoices USING btree (tenant_id);


-- Name: idx_notifications_user; Type: INDEX; Schema: public; Owner: -

CREATE INDEX idx_notifications_user ON public.notifications USING btree (user_id);


-- Name: idx_notifications_user_unread; Type: INDEX; Schema: public; Owner: -

CREATE INDEX idx_notifications_user_unread ON public.notifications USING btree (user_id, is_read) WHERE (is_read = false);


-- Name: idx_oauth_accounts_provider; Type: INDEX; Schema: public; Owner: -

CREATE INDEX idx_oauth_accounts_provider ON public.oauth_accounts USING btree (provider, provider_user_id);


-- Name: idx_oauth_accounts_user; Type: INDEX; Schema: public; Owner: -

CREATE INDEX idx_oauth_accounts_user ON public.oauth_accounts USING btree (user_id);


-- Name: idx_plan_features_plan; Type: INDEX; Schema: public; Owner: -

CREATE INDEX idx_plan_features_plan ON public.plan_features USING btree (plan_id);


-- Name: idx_plans_slug; Type: INDEX; Schema: public; Owner: -

CREATE INDEX idx_plans_slug ON public.plans USING btree (slug);


-- Name: idx_roles_name_global; Type: INDEX; Schema: public; Owner: -

CREATE UNIQUE INDEX idx_roles_name_global ON public.roles USING btree (name) WHERE (tenant_id IS NULL);


-- Name: idx_roles_tenant; Type: INDEX; Schema: public; Owner: -

CREATE INDEX idx_roles_tenant ON public.roles USING btree (tenant_id);


-- Name: idx_sessions_token; Type: INDEX; Schema: public; Owner: -

CREATE INDEX idx_sessions_token ON public.sessions USING btree (token);


-- Name: idx_settings_global_key; Type: INDEX; Schema: public; Owner: -

CREATE UNIQUE INDEX idx_settings_global_key ON public.settings USING btree (key) WHERE (tenant_id IS NULL);


-- Name: idx_settings_tenant; Type: INDEX; Schema: public; Owner: -

CREATE INDEX idx_settings_tenant ON public.settings USING btree (tenant_id);


-- Name: idx_settings_tenant_key; Type: INDEX; Schema: public; Owner: -

CREATE UNIQUE INDEX idx_settings_tenant_key ON public.settings USING btree (tenant_id, key) WHERE (tenant_id IS NOT NULL);


-- Name: idx_tenant_subscriptions_tenant; Type: INDEX; Schema: public; Owner: -

CREATE INDEX idx_tenant_subscriptions_tenant ON public.tenant_subscriptions USING btree (tenant_id);


-- Name: idx_tenants_slug; Type: INDEX; Schema: public; Owner: -

CREATE INDEX idx_tenants_slug ON public.tenants USING btree (slug);


-- Name: idx_trusted_devices_expires; Type: INDEX; Schema: public; Owner: -

CREATE INDEX idx_trusted_devices_expires ON public.trusted_devices USING btree (expires_at);


-- Name: idx_trusted_devices_user; Type: INDEX; Schema: public; Owner: -

CREATE INDEX idx_trusted_devices_user ON public.trusted_devices USING btree (user_id);


-- Name: idx_users_email; Type: INDEX; Schema: public; Owner: -

CREATE INDEX idx_users_email ON public.users USING btree (email);


-- Name: file_records file_records_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.file_records
    ADD CONSTRAINT file_records_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


-- Name: file_records file_records_uploaded_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.file_records
    ADD CONSTRAINT file_records_uploaded_by_fkey FOREIGN KEY (uploaded_by) REFERENCES public.users(id) ON DELETE SET NULL;


-- Name: invoices invoices_merchant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.invoices
    ADD CONSTRAINT invoices_merchant_id_fkey FOREIGN KEY (merchant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


-- Name: invoices invoices_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.invoices
    ADD CONSTRAINT invoices_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


-- Name: notification_preferences notification_preferences_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.notification_preferences
    ADD CONSTRAINT notification_preferences_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


-- Name: notifications notifications_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.notifications
    ADD CONSTRAINT notifications_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


-- Name: notifications notifications_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.notifications
    ADD CONSTRAINT notifications_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


-- Name: oauth_accounts oauth_accounts_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.oauth_accounts
    ADD CONSTRAINT oauth_accounts_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


-- Name: plan_features plan_features_feature_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.plan_features
    ADD CONSTRAINT plan_features_feature_id_fkey FOREIGN KEY (feature_id) REFERENCES public.features(id) ON DELETE CASCADE;


-- Name: plan_features plan_features_plan_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.plan_features
    ADD CONSTRAINT plan_features_plan_id_fkey FOREIGN KEY (plan_id) REFERENCES public.plans(id) ON DELETE CASCADE;


-- Name: push_subscriptions push_subscriptions_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.push_subscriptions
    ADD CONSTRAINT push_subscriptions_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


-- Name: role_permissions role_permissions_permission_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.role_permissions
    ADD CONSTRAINT role_permissions_permission_id_fkey FOREIGN KEY (permission_id) REFERENCES public.permissions(id) ON DELETE CASCADE;


-- Name: role_permissions role_permissions_role_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.role_permissions
    ADD CONSTRAINT role_permissions_role_id_fkey FOREIGN KEY (role_id) REFERENCES public.roles(id) ON DELETE CASCADE;


-- Name: roles roles_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.roles
    ADD CONSTRAINT roles_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


-- Name: sessions sessions_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.sessions
    ADD CONSTRAINT sessions_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


-- Name: sessions sessions_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.sessions
    ADD CONSTRAINT sessions_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


-- Name: settings settings_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.settings
    ADD CONSTRAINT settings_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


-- Name: tenant_members tenant_members_role_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.tenant_members
    ADD CONSTRAINT tenant_members_role_id_fkey FOREIGN KEY (role_id) REFERENCES public.roles(id);


-- Name: tenant_members tenant_members_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.tenant_members
    ADD CONSTRAINT tenant_members_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


-- Name: tenant_members tenant_members_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.tenant_members
    ADD CONSTRAINT tenant_members_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


-- Name: tenant_subscriptions tenant_subscriptions_plan_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.tenant_subscriptions
    ADD CONSTRAINT tenant_subscriptions_plan_id_fkey FOREIGN KEY (plan_id) REFERENCES public.plans(id);


-- Name: tenant_subscriptions tenant_subscriptions_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.tenant_subscriptions
    ADD CONSTRAINT tenant_subscriptions_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


-- Name: trusted_devices trusted_devices_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -

ALTER TABLE ONLY public.trusted_devices
    ADD CONSTRAINT trusted_devices_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;



