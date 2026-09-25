-- Down: sengaja TIDAK mengembalikan nilai rusak ('' dan tanpa max_users).
-- Mengembalikan data yang membuat tenant Enterprise terblokir menambah anggota
-- bukan sesuatu yang boleh dilakukan otomatis oleh rollback. Yang dibersihkan
-- hanyalah baris yang MURNI dibuat migrasi ini, yaitu `max_users` — fitur itu
-- sebelumnya tidak pernah ada di plan_features sama sekali.
DELETE FROM plan_features
WHERE feature_id IN (SELECT id FROM features WHERE code = 'max_users');
