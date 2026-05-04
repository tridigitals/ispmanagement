WITH default_templates (
    key,
    name,
    description,
    use_case,
    trigger_mode,
    event_key,
    channel,
    status,
    whatsapp_body,
    email_subject,
    email_body,
    variables
) AS (
    VALUES
    (
        'billing_payment_reminder',
        'Billing - Friendly Payment Reminder',
        'A polite manual reminder for customers who need a payment follow-up.',
        'billing',
        'manual',
        'billing.payment_reminder',
        'both',
        'active',
        $$Halo {{customer.name}}, kami dari {{tenant.name}} ingin mengingatkan tagihan layanan internet Anda.

Jika sudah melakukan pembayaran, abaikan pesan ini. Jika membutuhkan bantuan, balas pesan ini agar tim kami bisa membantu.$$,
        'Pengingat pembayaran layanan {{tenant.name}}',
        $$Halo {{customer.name}},

Kami dari {{tenant.name}} ingin mengingatkan tagihan layanan internet Anda.

Jika pembayaran sudah dilakukan, email ini dapat diabaikan. Jika ada kendala pembayaran atau membutuhkan bantuan, silakan hubungi tim kami melalui channel resmi.

Terima kasih,
{{tenant.name}}$$,
        '["tenant.name","customer.name"]'
    ),
    (
        'billing_overdue_followup',
        'Billing - Overdue Follow-up',
        'A firmer follow-up for overdue billing without sounding aggressive.',
        'billing',
        'manual',
        'billing.overdue_followup',
        'both',
        'active',
        $$Halo {{customer.name}}, kami dari {{tenant.name}} mencatat pembayaran layanan Anda masih perlu ditindaklanjuti.

Mohon lakukan pembayaran atau hubungi kami jika ada kendala, agar layanan tetap berjalan dengan baik.$$,
        'Tindak lanjut pembayaran layanan {{tenant.name}}',
        $$Halo {{customer.name}},

Kami mencatat pembayaran layanan Anda masih perlu ditindaklanjuti.

Mohon lakukan pembayaran melalui metode yang tersedia. Jika ada kendala, balas email ini atau hubungi tim kami agar kami dapat membantu pengecekan.

Terima kasih,
{{tenant.name}}$$,
        '["tenant.name","customer.name"]'
    ),
    (
        'installation_schedule_confirmation',
        'Installation - Schedule Confirmation',
        'Confirm installation readiness and keep the customer informed before field work.',
        'installation',
        'manual',
        'installation.schedule_confirmation',
        'whatsapp',
        'active',
        $$Halo {{customer.name}}, tim {{tenant.name}} akan menindaklanjuti jadwal instalasi layanan Anda.

Mohon pastikan lokasi dapat diakses dan nomor ini aktif untuk koordinasi teknisi. Jika ada perubahan jadwal, balas pesan ini.$$,
        NULL,
        NULL,
        '["tenant.name","customer.name"]'
    ),
    (
        'installation_completed',
        'Installation - Completed Handoff',
        'Send after installation is completed to guide the customer on the next step.',
        'installation',
        'manual',
        'installation.completed',
        'both',
        'active',
        $$Halo {{customer.name}}, instalasi layanan {{tenant.name}} sudah selesai.

Silakan coba koneksi internet Anda. Jika ada kendala, balas pesan ini agar tim kami dapat membantu pengecekan.$$,
        'Instalasi layanan {{tenant.name}} selesai',
        $$Halo {{customer.name}},

Instalasi layanan {{tenant.name}} sudah selesai.

Silakan coba koneksi internet Anda. Jika ada kendala setelah instalasi, hubungi tim kami dengan menjelaskan gejala yang dialami agar pengecekan dapat dilakukan lebih cepat.

Terima kasih,
{{tenant.name}}$$,
        '["tenant.name","customer.name"]'
    ),
    (
        'outage_customer_notice',
        'Outage - Customer Notice',
        'Notify customers about an incident while keeping the message calm and concise.',
        'outage',
        'manual',
        'network.outage_notice',
        'both',
        'active',
        $$Halo {{customer.name}}, saat ini tim {{tenant.name}} sedang menangani gangguan layanan di beberapa area.

Kami akan menginformasikan pembaruan berikutnya setelah pengecekan selesai. Terima kasih atas kesabarannya.$$,
        'Informasi gangguan layanan {{tenant.name}}',
        $$Halo {{customer.name}},

Saat ini tim {{tenant.name}} sedang menangani gangguan layanan di beberapa area.

Tim teknis sedang melakukan pengecekan dan kami akan menginformasikan pembaruan berikutnya setelah ada perkembangan. Terima kasih atas pengertian dan kesabarannya.

Hormat kami,
{{tenant.name}}$$,
        '["tenant.name","customer.name"]'
    ),
    (
        'support_followup',
        'Support - Follow-up Check',
        'A clean follow-up message after a support case is handled.',
        'support',
        'manual',
        'support.followup',
        'both',
        'active',
        $$Halo {{customer.name}}, kami dari {{tenant.name}} ingin memastikan kendala Anda sudah terbantu.

Jika masih ada masalah, balas pesan ini agar tim support dapat melanjutkan pengecekan.$$,
        'Follow-up bantuan dari {{tenant.name}}',
        $$Halo {{customer.name}},

Kami ingin memastikan kendala Anda sudah terbantu oleh tim {{tenant.name}}.

Jika masih ada masalah atau membutuhkan bantuan lanjutan, silakan balas email ini dengan detail kendala yang dialami.

Terima kasih,
{{tenant.name}}$$,
        '["tenant.name","customer.name"]'
    )
)
INSERT INTO public.message_templates (
    id,
    tenant_id,
    key,
    name,
    description,
    use_case,
    target,
    trigger_mode,
    event_key,
    channel,
    locale,
    status,
    whatsapp_body,
    email_subject,
    email_body,
    variables,
    version,
    created_at,
    updated_at
)
SELECT
    'seed_tpl_' || md5(t.id || ':' || d.key),
    t.id,
    d.key,
    d.name,
    d.description,
    d.use_case,
    'customer',
    d.trigger_mode,
    d.event_key,
    d.channel,
    'id-ID',
    d.status,
    d.whatsapp_body,
    d.email_subject,
    d.email_body,
    d.variables,
    1,
    now(),
    now()
FROM public.tenants t
CROSS JOIN default_templates d
ON CONFLICT (tenant_id, key) DO NOTHING;
