-- Customer portal users must not inherit the internal storage console permission.
-- Keep this correction append-only: prior migrations may already be applied.
DELETE FROM public.role_permissions AS rp
USING public.roles AS r, public.permissions AS p
WHERE rp.role_id = r.id
  AND rp.permission_id = p.id
  AND r.tenant_id IS NULL
  AND r.is_system = TRUE
  AND lower(r.name) = 'customer'
  AND p.resource = 'storage_console'
  AND p.action = 'read';

-- The statement is intentionally idempotent: rerunning it removes zero rows.
