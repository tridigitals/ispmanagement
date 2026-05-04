DELETE FROM public.message_templates
WHERE key IN (
    'billing_payment_reminder',
    'billing_overdue_followup',
    'installation_schedule_confirmation',
    'installation_completed',
    'outage_customer_notice',
    'support_followup'
)
AND id LIKE 'seed_tpl_%';
