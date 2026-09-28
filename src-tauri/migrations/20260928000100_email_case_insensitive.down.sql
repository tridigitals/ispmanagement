-- Balikkan email case-insensitive ke perilaku semula.
--
-- PENTING: penggabungan duplikat TIDAK dikembalikan. Baris `users` yang sudah
-- dihapus beserta rujukannya tidak bisa direkonstruksi dari sini. Yang
-- dikembalikan hanya bentuk constraint/index-nya.
--
-- Setelah down ini, `Dodo@x.com` dan `dodo@x.com` bisa kembali menjadi dua akun
-- dan yang ber-huruf-besar tidak bisa login — itulah perilaku yang diperbaiki.

DROP INDEX IF EXISTS idx_users_lower_email;
DROP INDEX IF EXISTS users_email_lower_key;

ALTER TABLE users ADD CONSTRAINT users_email_key UNIQUE (email);