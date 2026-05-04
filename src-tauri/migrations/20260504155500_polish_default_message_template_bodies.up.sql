UPDATE public.message_templates
SET
    channel = 'both',
    email_subject = 'Konfirmasi jadwal instalasi {{tenant.name}}',
    email_body = $$Halo {{customer.name}},

Tim {{tenant.name}} akan menindaklanjuti jadwal instalasi layanan Anda.

Mohon pastikan lokasi dapat diakses, ada PIC yang dapat ditemui, dan nomor kontak tetap aktif untuk koordinasi teknisi. Jika ada perubahan jadwal atau akses lokasi, silakan balas email ini agar tim kami dapat menyesuaikan kunjungan.

Terima kasih,
{{tenant.name}}$$,
    variables = '["tenant.name","customer.name"]',
    updated_at = now()
WHERE key = 'installation_schedule_confirmation'
  AND id LIKE 'seed_tpl_%';
