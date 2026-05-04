UPDATE public.message_templates
SET
    channel = 'whatsapp',
    email_subject = NULL,
    email_body = NULL,
    variables = '["tenant.name","customer.name"]',
    updated_at = now()
WHERE key = 'installation_schedule_confirmation'
  AND id LIKE 'seed_tpl_%';
