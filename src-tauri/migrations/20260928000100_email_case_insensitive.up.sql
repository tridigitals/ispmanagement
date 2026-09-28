-- Email case-insensitive: satu alamat = satu akun.
--
-- Masalah: `users.email` disimpan apa adanya, sementara `UNIQUE (email)` bawaan
-- Postgres bersifat CASE-SENSITIVE. Akibatnya:
--   * `Dodo@gmail.com` dan `dodo@gmail.com` menjadi DUA akun berbeda;
--   * login men-`to_lowercase()` input lalu mencari `WHERE email = $1`, sehingga
--     akun yang tersimpan dengan huruf besar TIDAK PERNAH ketemu — pemiliknya
--     tidak bisa masuk sama sekali, tanpa jalur pemulihan di UI;
--   * undangan/reset password untuk satu alamat bisa mengenai baris yang salah.
--
-- Migrasi ini melakukan tiga hal:
--   1. menggabungkan duplikat case-insensitive menjadi satu akun;
--   2. menyeragamkan email lama ke huruf kecil;
--   3. menegakkan invarian di level database lewat `UNIQUE (lower(email))`.
--
-- Langkah 3 penting karena memperbaiki aplikasi saja tidak cukup: kode masa
-- depan yang lupa menormalkan akan membuat duplikat lagi.

-- ---------------------------------------------------------------------------
-- 1. Gabungkan duplikat case-insensitive.
--
-- Baris yang DIPERTAHANKAN adalah yang paling awal dibuat (`created_at`),
-- karena biasanya itulah akun asli yang dipakai pemiliknya; baris berikutnya
-- yang paling mungkin dibuat karena kebingungan/duplikasi.
--
-- Perhatian: `users` menjadi target 22 foreign key (tenant_members,
-- customer_users, sessions, notifications, user_addresses, user_devices,
-- oauth_accounts, push_subscriptions, trusted_devices, technician_locations,
-- support_tickets, installation_work_orders, ...). Memindahkan `user_id`
-- sembarangan bisa melanggar unique constraint gabungan, jadi:
--   * `tenant_members` — jika duplikat sudah menjadi anggota tenant yang sama,
--     barisnya dihapus (bukan dipindah), sisanya dipindah. Ini juga yang
--     membuat "add anggota lalu dihapus lalu add lagi" tidak menumpuk.
--   * `customer_users` — dipindah hanya bila pasangan (tenant,user) dan
--     (customer,user) belum ada; kalau sudah ada, barisnya dihapus.
--   * `announcement_dismissals` — dipindah bila (user,announcement) belum ada.
--   * tabel lain tidak punya unique gabungan yang melibatkan `user_id`, jadi
--     pemindahan langsung aman.
-- ---------------------------------------------------------------------------

-- 1a. tenant_members: hapus yang akan bertabrakan, lalu pindahkan sisanya.
WITH dupes AS (
    SELECT ARRAY_AGG(u.id ORDER BY u.created_at, u.id) AS ids
    FROM users u
    GROUP BY lower(u.email)
    HAVING COUNT(*) > 1
), pairs AS (
    SELECT ids[1] AS keep_id, ids[2:] AS drop_ids FROM dupes
)
DELETE FROM tenant_members tm
USING pairs p
WHERE tm.user_id = ANY (p.drop_ids)
  AND EXISTS (
      SELECT 1 FROM tenant_members k
      WHERE k.tenant_id = tm.tenant_id AND k.user_id = p.keep_id
  );

WITH dupes AS (
    SELECT ARRAY_AGG(u.id ORDER BY u.created_at, u.id) AS ids
    FROM users u
    GROUP BY lower(u.email)
    HAVING COUNT(*) > 1
)
UPDATE tenant_members tm
SET user_id = d.ids[1]
FROM dupes d
WHERE tm.user_id = ANY (d.ids[2:]);

-- 1b. customer_users: hapus yang bertabrakan, lalu pindahkan.
WITH dupes AS (
    SELECT ARRAY_AGG(u.id ORDER BY u.created_at, u.id) AS ids
    FROM users u
    GROUP BY lower(u.email)
    HAVING COUNT(*) > 1
), pairs AS (
    SELECT ids[1] AS keep_id, ids[2:] AS drop_ids FROM dupes
)
DELETE FROM customer_users cu
USING pairs p
WHERE cu.user_id = ANY (p.drop_ids)
  AND (
      EXISTS (SELECT 1 FROM customer_users k
              WHERE k.user_id = p.keep_id AND k.tenant_id = cu.tenant_id)
      OR EXISTS (SELECT 1 FROM customer_users k
                 WHERE k.user_id = p.keep_id AND k.customer_id = cu.customer_id)
  );

WITH dupes AS (
    SELECT ARRAY_AGG(u.id ORDER BY u.created_at, u.id) AS ids
    FROM users u
    GROUP BY lower(u.email)
    HAVING COUNT(*) > 1
)
UPDATE customer_users cu
SET user_id = d.ids[1]
FROM dupes d
WHERE cu.user_id = ANY (d.ids[2:]);

-- 1c. announcement_dismissals: hapus yang bertabrakan, lalu pindahkan.
WITH dupes AS (
    SELECT ARRAY_AGG(u.id ORDER BY u.created_at, u.id) AS ids
    FROM users u
    GROUP BY lower(u.email)
    HAVING COUNT(*) > 1
), pairs AS (
    SELECT ids[1] AS keep_id, ids[2:] AS drop_ids FROM dupes
)
DELETE FROM announcement_dismissals ad
USING pairs p
WHERE ad.user_id = ANY (p.drop_ids)
  AND EXISTS (SELECT 1 FROM announcement_dismissals k
              WHERE k.user_id = p.keep_id AND k.announcement_id = ad.announcement_id);

WITH dupes AS (
    SELECT ARRAY_AGG(u.id ORDER BY u.created_at, u.id) AS ids
    FROM users u
    GROUP BY lower(u.email)
    HAVING COUNT(*) > 1
)
UPDATE announcement_dismissals ad
SET user_id = d.ids[1]
FROM dupes d
WHERE ad.user_id = ANY (d.ids[2:]);

-- 1d. notification_preferences / trusted_devices / user_devices / sessions /
--     oauth_accounts / push_subscriptions / user_addresses: pindahkan langsung.
WITH dupes AS (
    SELECT ARRAY_AGG(u.id ORDER BY u.created_at, u.id) AS ids
    FROM users u
    GROUP BY lower(u.email)
    HAVING COUNT(*) > 1
)
UPDATE notification_preferences t SET user_id = d.ids[1]
FROM dupes d WHERE t.user_id = ANY (d.ids[2:]);

WITH dupes AS (
    SELECT ARRAY_AGG(u.id ORDER BY u.created_at, u.id) AS ids
    FROM users u
    GROUP BY lower(u.email)
    HAVING COUNT(*) > 1
)
UPDATE trusted_devices t SET user_id = d.ids[1]
FROM dupes d WHERE t.user_id = ANY (d.ids[2:]);

WITH dupes AS (
    SELECT ARRAY_AGG(u.id ORDER BY u.created_at, u.id) AS ids
    FROM users u
    GROUP BY lower(u.email)
    HAVING COUNT(*) > 1
)
UPDATE user_devices t SET user_id = d.ids[1]
FROM dupes d WHERE t.user_id = ANY (d.ids[2:]);

WITH dupes AS (
    SELECT ARRAY_AGG(u.id ORDER BY u.created_at, u.id) AS ids
    FROM users u
    GROUP BY lower(u.email)
    HAVING COUNT(*) > 1
)
UPDATE sessions t SET user_id = d.ids[1]
FROM dupes d WHERE t.user_id = ANY (d.ids[2:]);

WITH dupes AS (
    SELECT ARRAY_AGG(u.id ORDER BY u.created_at, u.id) AS ids
    FROM users u
    GROUP BY lower(u.email)
    HAVING COUNT(*) > 1
)
UPDATE oauth_accounts t SET user_id = d.ids[1]
FROM dupes d WHERE t.user_id = ANY (d.ids[2:]);

WITH dupes AS (
    SELECT ARRAY_AGG(u.id ORDER BY u.created_at, u.id) AS ids
    FROM users u
    GROUP BY lower(u.email)
    HAVING COUNT(*) > 1
)
UPDATE push_subscriptions t SET user_id = d.ids[1]
FROM dupes d WHERE t.user_id = ANY (d.ids[2:]);

WITH dupes AS (
    SELECT ARRAY_AGG(u.id ORDER BY u.created_at, u.id) AS ids
    FROM users u
    GROUP BY lower(u.email)
    HAVING COUNT(*) > 1
)
UPDATE user_addresses t SET user_id = d.ids[1]
FROM dupes d WHERE t.user_id = ANY (d.ids[2:]);

-- 1e. Kolom penunjuk lain yang mereferensi users (nullable, SET NULL).
WITH dupes AS (
    SELECT ARRAY_AGG(u.id ORDER BY u.created_at, u.id) AS ids
    FROM users u
    GROUP BY lower(u.email)
    HAVING COUNT(*) > 1
)
UPDATE technician_locations t SET technician_id = d.ids[1]
FROM dupes d WHERE t.technician_id = ANY (d.ids[2:]);

WITH dupes AS (
    SELECT ARRAY_AGG(u.id ORDER BY u.created_at, u.id) AS ids
    FROM users u
    GROUP BY lower(u.email)
    HAVING COUNT(*) > 1
)
UPDATE support_tickets t SET assigned_to = d.ids[1]
FROM dupes d WHERE t.assigned_to = ANY (d.ids[2:]);

WITH dupes AS (
    SELECT ARRAY_AGG(u.id ORDER BY u.created_at, u.id) AS ids
    FROM users u
    GROUP BY lower(u.email)
    HAVING COUNT(*) > 1
)
UPDATE support_tickets t SET created_by = d.ids[1]
FROM dupes d WHERE t.created_by = ANY (d.ids[2:]);

WITH dupes AS (
    SELECT ARRAY_AGG(u.id ORDER BY u.created_at, u.id) AS ids
    FROM users u
    GROUP BY lower(u.email)
    HAVING COUNT(*) > 1
)
UPDATE support_ticket_messages t SET author_id = d.ids[1]
FROM dupes d WHERE t.author_id = ANY (d.ids[2:]);

WITH dupes AS (
    SELECT ARRAY_AGG(u.id ORDER BY u.created_at, u.id) AS ids
    FROM users u
    GROUP BY lower(u.email)
    HAVING COUNT(*) > 1
)
UPDATE installation_work_orders t SET assigned_to = d.ids[1]
FROM dupes d WHERE t.assigned_to = ANY (d.ids[2:]);

WITH dupes AS (
    SELECT ARRAY_AGG(u.id ORDER BY u.created_at, u.id) AS ids
    FROM users u
    GROUP BY lower(u.email)
    HAVING COUNT(*) > 1
)
UPDATE customer_registration_invites t SET created_by = d.ids[1]
FROM dupes d WHERE t.created_by = ANY (d.ids[2:]);

WITH dupes AS (
    SELECT ARRAY_AGG(u.id ORDER BY u.created_at, u.id) AS ids
    FROM users u
    GROUP BY lower(u.email)
    HAVING COUNT(*) > 1
)
UPDATE mixradius_import_batches t SET created_by = d.ids[1]
FROM dupes d WHERE t.created_by = ANY (d.ids[2:]);

WITH dupes AS (
    SELECT ARRAY_AGG(u.id ORDER BY u.created_at, u.id) AS ids
    FROM users u
    GROUP BY lower(u.email)
    HAVING COUNT(*) > 1
)
UPDATE file_records t SET uploaded_by = d.ids[1]
FROM dupes d WHERE t.uploaded_by = ANY (d.ids[2:]);

WITH dupes AS (
    SELECT ARRAY_AGG(u.id ORDER BY u.created_at, u.id) AS ids
    FROM users u
    GROUP BY lower(u.email)
    HAVING COUNT(*) > 1
)
UPDATE users t SET rejected_by_user_id = d.ids[1]
FROM dupes d WHERE t.rejected_by_user_id = ANY (d.ids[2:]);

WITH dupes AS (
    SELECT ARRAY_AGG(u.id ORDER BY u.created_at, u.id) AS ids
    FROM users u
    GROUP BY lower(u.email)
    HAVING COUNT(*) > 1
)
UPDATE users t SET approved_by_user_id = d.ids[1]
FROM dupes d WHERE t.approved_by_user_id = ANY (d.ids[2:]);

-- 1f. Hapus baris user duplikat (setelah semua rujukan dipindahkan).
WITH dupes AS (
    SELECT ARRAY_AGG(u.id ORDER BY u.created_at, u.id) AS ids
    FROM users u
    GROUP BY lower(u.email)
    HAVING COUNT(*) > 1
)
DELETE FROM users u
WHERE u.id IN (SELECT unnest(d.ids[2:]) FROM dupes d);

-- ---------------------------------------------------------------------------
-- 2. Seragamkan email lama ke huruf kecil + trim.
-- ---------------------------------------------------------------------------
UPDATE users SET email = lower(btrim(email)) WHERE email <> lower(btrim(email));

-- ---------------------------------------------------------------------------
-- 3. Tegakkan invarian di level database.
--
-- Ganti UNIQUE (email) yang case-sensitive dengan UNIQUE (lower(email)).
-- `users_email_key` adalah constraint (bukan index biasa), jadi perlu di-DROP
-- sebagai constraint.
-- ---------------------------------------------------------------------------
ALTER TABLE users DROP CONSTRAINT IF EXISTS users_email_key;
DROP INDEX IF EXISTS users_email_lower_key;
CREATE UNIQUE INDEX users_email_lower_key ON users (lower(email));

-- Index pencarian case-insensitive yang dipakai login/registrasi.
CREATE INDEX IF NOT EXISTS idx_users_lower_email ON users (lower(email));
