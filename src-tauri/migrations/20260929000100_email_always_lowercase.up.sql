-- Email SELALU tersimpan huruf kecil — ditegakkan database, bukan hanya aplikasi.
--
-- ## Kenapa
--
-- Normalisasi di sisi aplikasi saja tidak cukup: ada 20+ jalur tulis yang
-- menyentuh kolom email di berbagai service, dan satu jalur yang lupa (atau kode
-- baru enam bulan lagi) langsung membuat data kembali bercampur huruf besar.
-- Sementara itu banyak pencarian dan pembandingan memakai `lower(email)`, jadi
-- campuran huruf besar/kecil adalah sumber bug yang sulit terlihat:
--   * `Alfamipratama41@gmail.com` dan `alfamipratama41@gmail.com` tampak "beda"
--     bagi kode yang membandingkan tanpa `lower()`;
--   * laporan/ekspor jadi tidak konsisten;
--   * pencocokan data pelanggan antar sistem (import, sinkronisasi akun) gagal
--     diam-diam.
--
-- Perbaikan sebelumnya (20260928000100) menegakkan `UNIQUE (lower(email))` untuk
-- `users` — itu mencegah DUPLIKAT, tapi tidak membuat satu alamat selalu
-- tersimpan dalam satu bentuk. Migrasi ini melengkapinya: apa pun yang ditulis
-- aplikasi, yang tersimpan tetap huruf kecil.
--
-- ## Tabel yang disentuh
--
-- Dipilih dari katalog (semua kolom email TEXT yang menyimpan alamat email):
--   * `customers.email`           — alamat email pelanggan (dipakai portal/login)
--   * `email_outbox.to_email`     — tujuan pengiriman email (keputusan retry)
--   * `oauth_accounts.provider_email` — email dari penyedia OAuth (pencocokan akun)
--   * `mixradius_staging_customers.email` — data import; dipakai mencocokkan
--     pelanggan yang sudah ada, jadi normalisasi ikut memperbaiki pencocokan
--
-- `users.email` TIDAK dimasukkan (sudah punya invarian sendiri + `UNIQUE
-- (lower(email))` dari migrasi sebelumnya; menambah trigger di sana tidak
-- menambah jaminan apa pun).
--
-- ## Kenapa trigger, bukan hanya `UPDATE`
--
-- `UPDATE` di bawah hanya membersihkan data yang ADA SEKARANG. Trigger yang
-- membuat invarian ini bertahan untuk semua tulisan berikutnya. Karena itu
-- keduanya ada di satu migrasi.

-- ---------------------------------------------------------------------------
-- 1. Fungsi normalisasi email — generik, satu fungsi untuk semua tabel.
--
--    Ditulis generik supaya tidak ada salinan logika per tabel yang bisa
--    menyimpang. Kolom yang dinormalkan diberikan lewat argumen trigger.
--
--    Memakai jsonb supaya tidak perlu `NEW.<kolom>` literal: nama kolomnya
--    dinamis. Jalur ini dipakai HANYA saat nilainya memang perlu berubah, jadi
--    biayanya nihil untuk sebagian besar tulisan.
-- ---------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION public.tg_lower_email_column()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
  col text := TG_ARGV[0];
  rec jsonb;
  val text;
BEGIN
  rec := to_jsonb(NEW);
  val := rec ->> col;

  IF val IS NOT NULL THEN
    val := lower(btrim(val));
    IF (rec ->> col) <> val THEN
      rec := jsonb_set(rec, ARRAY[col], to_jsonb(val), false);
      NEW := jsonb_populate_record(NEW, rec);
    END IF;
  END IF;

  RETURN NEW;
END;
$$;

COMMENT ON FUNCTION public.tg_lower_email_column() IS
  'Trigger helper: trim + lowercase satu kolom email. Pakai EXECUTE FUNCTION public.tg_lower_email_column(''nama_kolom'').';

-- ---------------------------------------------------------------------------
-- 2. Seragamkan data lama ke huruf kecil.
--
--    Trim sekaligus: ` abc@x.com ` dan `abc@x.com` adalah alamat yang sama, dan
--    kalau tidak ditrim sekarang, trigger di langkah 3 hanya akan rapi untuk
--    tulisan berikutnya — sisa data lama tetap bercampur.
-- ---------------------------------------------------------------------------
UPDATE customers SET email = lower(btrim(email))
WHERE email IS NOT NULL AND email <> lower(btrim(email));

UPDATE email_outbox SET to_email = lower(btrim(to_email))
WHERE to_email IS NOT NULL AND to_email <> lower(btrim(to_email));

UPDATE oauth_accounts SET provider_email = lower(btrim(provider_email))
WHERE provider_email IS NOT NULL AND provider_email <> lower(btrim(provider_email));

UPDATE mixradius_staging_customers SET email = lower(btrim(email))
WHERE email IS NOT NULL AND email <> lower(btrim(email));

-- ---------------------------------------------------------------------------
-- 3. Tegakkan invarian untuk semua tulisan berikutnya.
--
--    Sekaligus memastikan data lama yang hurufnya campur tidak bisa masuk lagi
--    lewat jalur mana pun (API, import, perbaikan manual di DB).
-- ---------------------------------------------------------------------------
DROP TRIGGER IF EXISTS trg_customers_lower_email ON public.customers;
CREATE TRIGGER trg_customers_lower_email
BEFORE INSERT OR UPDATE OF email ON public.customers
FOR EACH ROW EXECUTE FUNCTION public.tg_lower_email_column('email');

DROP TRIGGER IF EXISTS trg_email_outbox_lower_email ON public.email_outbox;
CREATE TRIGGER trg_email_outbox_lower_email
BEFORE INSERT OR UPDATE OF to_email ON public.email_outbox
FOR EACH ROW EXECUTE FUNCTION public.tg_lower_email_column('to_email');

DROP TRIGGER IF EXISTS trg_oauth_accounts_lower_email ON public.oauth_accounts;
CREATE TRIGGER trg_oauth_accounts_lower_email
BEFORE INSERT OR UPDATE OF provider_email ON public.oauth_accounts
FOR EACH ROW EXECUTE FUNCTION public.tg_lower_email_column('provider_email');

DROP TRIGGER IF EXISTS trg_mixradius_staging_lower_email ON public.mixradius_staging_customers;
CREATE TRIGGER trg_mixradius_staging_lower_email
BEFORE INSERT OR UPDATE OF email ON public.mixradius_staging_customers
FOR EACH ROW EXECUTE FUNCTION public.tg_lower_email_column('email');
