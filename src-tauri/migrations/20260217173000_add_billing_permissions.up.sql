-- Add dedicated billing permissions and assign them to default system roles.

INSERT INTO public.permissions (id, resource, action, description)
VALUES
    ('perm_billing_read', 'billing', 'read', 'View billing and subscription data'),
    ('perm_billing_manage', 'billing', 'manage', 'Manage billing actions')
ON CONFLICT (resource, action) DO UPDATE
SET description = EXCLUDED.description;

INSERT INTO public.role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM public.roles r
JOIN public.permissions p
  ON p.resource = 'billing'
 AND p.action = 'read'
WHERE r.is_system = true
  AND r.name IN ('Owner', 'Admin')
  AND NOT EXISTS (
      SELECT 1
      FROM public.role_permissions rp
      WHERE rp.role_id = r.id
        AND rp.permission_id = p.id
  );

INSERT INTO public.role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM public.roles r
JOIN public.permissions p
  ON p.resource = 'billing'
 AND p.action = 'manage'
WHERE r.is_system = true
  AND r.name IN ('Owner', 'Admin')
  AND NOT EXISTS (
      SELECT 1
      FROM public.role_permissions rp
      WHERE rp.role_id = r.id
        AND rp.permission_id = p.id
  );
