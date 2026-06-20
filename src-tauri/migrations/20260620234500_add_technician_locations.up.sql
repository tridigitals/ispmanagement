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

-- RLS disabled for now — tenant scoping enforced in Rust handlers.
-- To enable RLS later (e.g. Supabase), add policies using auth.uid() helper.
-- ALTER TABLE public.technician_locations ENABLE ROW LEVEL SECURITY;