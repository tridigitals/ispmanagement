-- Rollback dedicated billing permissions assignment.

DELETE FROM public.role_permissions rp
USING public.permissions p
WHERE rp.permission_id = p.id
  AND p.resource = 'billing'
  AND p.action IN ('read', 'manage');

DELETE FROM public.permissions
WHERE resource = 'billing'
  AND action IN ('read', 'manage');
