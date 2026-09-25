-- Perbaikan nilai plan_features yang rusak di DB yang sudah berjalan.
--
-- Latar: `seed_plans` memakai ON CONFLICT DO NOTHING dan tidak ada migrasi
-- yang pernah menulis plans/features/plan_features. Akibatnya memperbaiki
-- katalog di kode TIDAK berpengaruh pada instalasi yang sudah ada — dan tiga
-- kerusakan nyata ditemukan di produksi:
--
-- 1. `max_users` tidak pernah di-set untuk plan mana pun → semua plan jatuh ke
--    default global 5. Saat ini dua tenant Enterprise produksi punya 6 dan 7
--    anggota, jadi keduanya akan DITOLAK saat menambah anggota tim
--    (team_service: "Plan limit reached: Maximum 5 users allowed.").
-- 2. `max_members` dan `max_storage_gb` Enterprise tersimpan sebagai string
--    KOSONG (''), bukan NULL. Pembaca limit lama (`parse::<i64>()`) gagal
--    parse lalu memperlakukannya sebagai TANPA BATAS, sementara `is_truthy('')`
--    menganggapnya false — dua pembaca menafsirkan baris yang sama secara
--    berlawanan.
-- 3. `max_storage_gb` Free = '0.5' juga gagal parse → tanpa batas.
--
-- Isi nilai diselaraskan dengan `services::plan_catalog::plan_feature_values`.
-- Idempotent: aman dijalankan berulang, dan hanya menulis baris yang memang
-- sudah ada (insert baris baru adalah tugas seed, bukan migrasi).

-- 1) max_users: SEMUA plan (free/pro/enterprise) — inilah yang memblokir tenant.
--    Enterprise memakai 'unlimited' supaya jumlah anggota yang sudah ada
--    (6-7) tidak terkunci.
INSERT INTO plan_features (id, plan_id, feature_id, value)
SELECT gen_random_uuid()::text, p.id, f.id, v.val
FROM plans p
CROSS JOIN features f
CROSS JOIN (VALUES ('free', '3'), ('pro', '20'), ('enterprise', 'unlimited')) AS v(slug, val)
WHERE p.slug = v.slug AND f.code = 'max_users'
ON CONFLICT (plan_id, feature_id) DO UPDATE SET value = EXCLUDED.value;

-- 2) max_members: free=2, pro=10, enterprise=unlimited
INSERT INTO plan_features (id, plan_id, feature_id, value)
SELECT gen_random_uuid()::text, p.id, f.id, v.val
FROM plans p
CROSS JOIN features f
CROSS JOIN (VALUES ('free', '2'), ('pro', '10'), ('enterprise', 'unlimited')) AS v(slug, val)
WHERE p.slug = v.slug AND f.code = 'max_members'
ON CONFLICT (plan_id, feature_id) DO UPDATE SET value = EXCLUDED.value;

-- 3) max_storage_gb: free=1, pro=50, enterprise=500 (Enterprise dari '' → 500)
INSERT INTO plan_features (id, plan_id, feature_id, value)
SELECT gen_random_uuid()::text, p.id, f.id, v.val
FROM plans p
CROSS JOIN features f
CROSS JOIN (VALUES ('free', '1'), ('pro', '50'), ('enterprise', '500')) AS v(slug, val)
WHERE p.slug = v.slug AND f.code = 'max_storage_gb'
ON CONFLICT (plan_id, feature_id) DO UPDATE SET value = EXCLUDED.value;

-- 4) Fitur boolean yang di-enforce: tetapkan eksplisit per plan supaya tidak
--    bergantung pada default_value global yang bisa berubah.
INSERT INTO plan_features (id, plan_id, feature_id, value)
SELECT gen_random_uuid()::text, p.id, f.id, v.val
FROM plans p
CROSS JOIN features f
CROSS JOIN (VALUES
    ('free',       'custom_domain',   'false'),
    ('free',       'api_access',      'false'),
    ('free',       'audit_logs',      'false'),
    ('free',       'managed_radius',  'false'),
    ('free',       'sso_support',     'false'),
    ('free',       'remove_branding', 'false'),
    ('free',       'support_level',   'community'),
    ('pro',        'custom_domain',   'true'),
    ('pro',        'api_access',      'false'),
    ('pro',        'audit_logs',      'false'),
    ('pro',        'managed_radius',  'false'),
    ('pro',        'sso_support',     'false'),
    ('pro',        'remove_branding', 'false'),
    ('pro',        'support_level',   'priority'),
    ('enterprise', 'custom_domain',   'true'),
    ('enterprise', 'api_access',      'true'),
    ('enterprise', 'audit_logs',      'true'),
    ('enterprise', 'managed_radius',  'true'),
    ('enterprise', 'sso_support',     'true'),
    ('enterprise', 'remove_branding', 'true'),
    ('enterprise', 'support_level',   'dedicated')
) AS v(slug, code, val)
WHERE p.slug = v.slug AND f.code = v.code
ON CONFLICT (plan_id, feature_id) DO UPDATE SET value = EXCLUDED.value;
