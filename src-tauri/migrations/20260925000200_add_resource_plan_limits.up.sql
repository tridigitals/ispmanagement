-- Limit resource per-plan (pelanggan / router / OLT) + pindahkan api_access &
-- audit_logs ke Pro.
--
-- Latar: satu-satunya satuan yang dibatasi sebelumnya adalah akun tim
-- (`max_users`) dan storage — tidak relevan untuk menentukan skala sebuah ISP.
-- Akibatnya tenant dengan 10 pelanggan dan 10.000 pelanggan membayar harga yang
-- sama. Tiga fitur baru ini menutup celah tersebut:
--
--   max_customers  free 25  | pro 300       | enterprise unlimited
--   max_routers    free 1   | pro 10        | enterprise 25
--   max_olts       free 0   | pro 3         | enterprise unlimited
--
-- Enterprise sengaja `unlimited` untuk pelanggan/OLT dan 25 router supaya tenant
-- terbesar yang sudah ada (548 pelanggan, 3 router, 3 OLT) tidak pernah terkunci
-- oleh limitnya sendiri.
--
-- Perubahan lain: `api_access` dan `audit_logs` dinaikkan dari Enterprise ke Pro
-- karena Pro sebelumnya hanya punya pembeda yang tidak di-enforce. `managed_radius`
-- tetap Enterprise-only (pembeda terkuat, di-enforce di 4 tempat).
--
-- Idempotent: aman dijalankan berulang. Baris fitur baru dibuat di sini karena
-- migrasi tidak boleh mengandalkan seed (`seed_default_features` berjalan setelah
-- migrasi dan hanya menulis bila baris belum ada).

-- 1) Daftarkan tiga fitur baru bila belum ada.
INSERT INTO features (id, code, name, description, value_type, category, default_value, sort_order, created_at)
SELECT gen_random_uuid()::text, v.code, v.name, v.description, 'number', 'limits', v.default_value, v.sort_order, now()
FROM (VALUES
    ('max_customers', 'Customer Limit', 'Maximum number of customers allowed (enforced when creating a customer)', '25', 3),
    ('max_routers',   'Router Limit',   'Maximum number of MikroTik routers allowed (enforced when adding a router)', '1', 4),
    ('max_olts',      'OLT Limit',      'Maximum number of OLT devices allowed (enforced when adding an OLT)', '0', 5)
) AS v(code, name, description, default_value, sort_order)
WHERE NOT EXISTS (SELECT 1 FROM features f WHERE f.code = v.code);

-- 2) Bila fitur dibuat oleh seeder pada deployment lain, rapikan definisinya.
UPDATE features SET
    name = v.name, description = v.description, value_type = 'number',
    category = 'limits', default_value = v.default_value, sort_order = v.sort_order
FROM (VALUES
    ('max_customers', 'Customer Limit', 'Maximum number of customers allowed (enforced when creating a customer)', '25', 3),
    ('max_routers',   'Router Limit',   'Maximum number of MikroTik routers allowed (enforced when adding a router)', '1', 4),
    ('max_olts',      'OLT Limit',      'Maximum number of OLT devices allowed (enforced when adding an OLT)', '0', 5)
) AS v(code, name, description, default_value, sort_order)
WHERE features.code = v.code;

-- 3) Nilai per plan untuk ketiga fitur baru.
INSERT INTO plan_features (id, plan_id, feature_id, value)
SELECT gen_random_uuid()::text, p.id, f.id, v.val
FROM plans p
CROSS JOIN features f
CROSS JOIN (VALUES
    ('free', 'max_customers', '25'), ('pro', 'max_customers', '300'), ('enterprise', 'max_customers', 'unlimited'),
    ('free', 'max_routers',   '1'),  ('pro', 'max_routers',   '10'),  ('enterprise', 'max_routers',   '25'),
    ('free', 'max_olts',      '0'),  ('pro', 'max_olts',      '3'),   ('enterprise', 'max_olts',      'unlimited')
) AS v(slug, code, val)
WHERE p.slug = v.slug AND f.code = v.code
ON CONFLICT (plan_id, feature_id) DO UPDATE SET value = EXCLUDED.value;

-- 4) api_access & audit_logs → Pro juga (pembeda Pro yang benar-benar di-enforce).
INSERT INTO plan_features (id, plan_id, feature_id, value)
SELECT gen_random_uuid()::text, p.id, f.id, 'true'
FROM plans p
CROSS JOIN features f
WHERE p.slug = 'pro' AND f.code IN ('api_access', 'audit_logs')
ON CONFLICT (plan_id, feature_id) DO UPDATE SET value = EXCLUDED.value;
