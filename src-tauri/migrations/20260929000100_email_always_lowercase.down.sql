-- Lepas penegakan "email selalu huruf kecil".
--
-- PENTING: data yang sudah tersimpan huruf kecil TIDAK dikembalikan ke bentuk
-- aslinya (bentuk asli memang tidak pernah disimpan — itu intinya). Yang
-- dilepas hanya trigger-nya, sehingga tulisan berikutnya kembali tersimpan apa
-- adanya seperti sebelum migrasi ini.
--
-- Fungsi helper ikut dihapus karena tidak ada trigger lain yang memakainya.
-- Kalau nanti butuh lagi, cukup jalankan migrasi up-nya.

DROP TRIGGER IF EXISTS trg_customers_lower_email ON public.customers;
DROP TRIGGER IF EXISTS trg_email_outbox_lower_email ON public.email_outbox;
DROP TRIGGER IF EXISTS trg_oauth_accounts_lower_email ON public.oauth_accounts;
DROP TRIGGER IF EXISTS trg_mixradius_staging_lower_email ON public.mixradius_staging_customers;

DROP FUNCTION IF EXISTS public.tg_lower_email_column();