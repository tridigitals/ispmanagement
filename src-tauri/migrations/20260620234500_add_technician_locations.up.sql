-- Sprint 2: technician_locations table for GPS tracking
-- Stores periodic location pings from mobile-technician app.
-- Admin can query latest position per technician for real-time map.
-- Indexed on (technician_id, captured_at DESC) for fast latest-position lookup.

CREATE TABLE IF NOT EXISTS public.technician_locations (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       uuid NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    technician_id   uuid NOT NULL REFERENCES public.users(id) ON DELETE CASCADE,
    latitude        double precision NOT NULL,
    longitude       double precision NOT NULL,
    accuracy        double precision,                 -- meters (from GPS)
    altitude        double precision,                 -- meters (optional)
    bearing         double precision,                 -- degrees 0-360 (optional)
    speed           double precision,                 -- m/s (optional)
    captured_at     timestamptz NOT NULL,             -- when phone got the fix
    sent_at         timestamptz NOT NULL DEFAULT now(), -- when API received
    battery_level   smallint                          -- 0-100 (optional)
);

-- Index for latest-position-per-technician queries (admin map view)
CREATE INDEX IF NOT EXISTS idx_technician_locations_tech_captured
    ON public.technician_locations (technician_id, captured_at DESC);

-- Index for tenant-scoped queries
CREATE INDEX IF NOT EXISTS idx_technician_locations_tenant_captured
    ON public.technician_locations (tenant_id, captured_at DESC);

-- RLS: technicians can INSERT their own rows, admin can SELECT all in tenant
ALTER TABLE public.technician_locations ENABLE ROW LEVEL SECURITY;

-- Drop existing policies if they exist (for idempotent re-runs)
DROP POLICY IF EXISTS technician_locations_insert_self ON public.technician_locations;
DROP POLICY IF EXISTS technician_locations_select_tenant ON public.technician_locations;

-- INSERT: only own user_id
CREATE POLICY technician_locations_insert_self ON public.technician_locations
    FOR INSERT
    WITH CHECK (technician_id = auth.uid());

-- SELECT: only same tenant
CREATE POLICY technician_locations_select_tenant ON public.technician_locations
    FOR SELECT
    USING (
        tenant_id = (
            SELECT tenant_id FROM public.users WHERE id = auth.uid()
        )
    );