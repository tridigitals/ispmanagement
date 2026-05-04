DELETE FROM public.role_permissions
WHERE permission_id IN (
    SELECT id FROM public.permissions WHERE resource = 'communication_templates'
);

DELETE FROM public.permissions WHERE resource = 'communication_templates';

DROP TABLE IF EXISTS public.message_templates;
