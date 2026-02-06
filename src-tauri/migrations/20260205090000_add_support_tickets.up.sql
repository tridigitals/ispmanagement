-- Support Tickets (multi-tenant)

CREATE TABLE IF NOT EXISTS public.support_tickets (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    created_by text REFERENCES public.users(id) ON DELETE SET NULL,
    subject text NOT NULL,
    status text NOT NULL DEFAULT 'open',
    priority text NOT NULL DEFAULT 'normal',
    assigned_to text REFERENCES public.users(id) ON DELETE SET NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    closed_at timestamp with time zone
);

CREATE INDEX IF NOT EXISTS idx_support_tickets_tenant_created_at
    ON public.support_tickets (tenant_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_support_tickets_tenant_status
    ON public.support_tickets (tenant_id, status, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_support_tickets_created_by
    ON public.support_tickets (created_by, updated_at DESC);

CREATE TABLE IF NOT EXISTS public.support_ticket_messages (
    id text PRIMARY KEY NOT NULL,
    ticket_id text NOT NULL REFERENCES public.support_tickets(id) ON DELETE CASCADE,
    author_id text REFERENCES public.users(id) ON DELETE SET NULL,
    body text NOT NULL,
    is_internal boolean NOT NULL DEFAULT false,
    created_at timestamp with time zone NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_support_ticket_messages_ticket_created_at
    ON public.support_ticket_messages (ticket_id, created_at ASC);

-- Minimal integrity checks via CHECK constraints (kept simple for portability)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'support_tickets_status_check'
    ) THEN
        ALTER TABLE public.support_tickets
            ADD CONSTRAINT support_tickets_status_check
            CHECK (status IN ('open', 'pending', 'closed'));
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'support_tickets_priority_check'
    ) THEN
        ALTER TABLE public.support_tickets
            ADD CONSTRAINT support_tickets_priority_check
            CHECK (priority IN ('low', 'normal', 'high', 'urgent'));
    END IF;
END $$;

