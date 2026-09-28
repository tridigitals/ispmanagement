-- Down: hapus limit resource yang ditambahkan migrasi ini.
-- `api_access`/`audit_logs` untuk Pro TIDAK dikembalikan ke false — menurunkan
-- kembali fitur yang sudah diberikan bukan sesuatu yang boleh dilakukan rollback
-- otomatis; ubah lewat UI plan bila memang diinginkan.

DELETE FROM plan_features
WHERE feature_id IN (SELECT id FROM features WHERE code IN ('max_customers', 'max_routers', 'max_olts'));

DELETE FROM features WHERE code IN ('max_customers', 'max_routers', 'max_olts');
