//! Katalog fitur plan — SATU sumber kebenaran.
//!
//! Sebelum ini definisi fitur ditulis di DUA tempat yang berbeda dan sudah
//! menyimpang:
//!
//! - `db/connection/seed.rs::seed_plans` — nama/deskripsi lama, tanpa
//!   `category`/`sort_order`, `max_storage_gb` default `"0.5"`, dan
//!   `support_level` bertipe `"string"` (tidak cocok dengan nilai
//!   komunitas/priority/dedicated yang sebenarnya deskriptif).
//! - `services/plan_service.rs::seed_default_features` — nama/deskripsi lebih
//!   baru, lengkap dengan `category` + `sort_order`, `max_storage_gb` default
//!   `"1"`, `support_level` bertipe `"text"`.
//!
//! Dua penulis dengan `code` yang sama menghasilkan baris yang saling
//! menimpa: mana yang menang bergantung pada urutan pemanggilan, sehingga
//! kategori/tipologi bisa berbeda antar-deployment. Katalog ini dipakai
//! bersama oleh kedua jalur supaya hasilnya identik.
//!
//! Urutan `sort_order` menentukan urutan tampil di editor plan superadmin
//! (halaman Superadmin → Plans → Features & Limits).

/// Definisi satu fitur plan.
pub struct FeatureDef {
    pub code: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    /// `boolean` | `number` | `text` — menentukan kontrol di editor plan.
    pub value_type: &'static str,
    pub category: &'static str,
    /// Nilai yang dipakai bila plan tidak punya baris `plan_features`.
    /// Juga fallback untuk tenant tanpa subscription.
    pub default_value: &'static str,
    pub sort_order: i32,
}

/// Definisi plan (Free/Pro/Enterprise).
pub struct PlanDef {
    pub name: &'static str,
    pub slug: &'static str,
    pub description: &'static str,
    pub price_monthly: f64,
    pub price_yearly: f64,
    pub is_active: bool,
    pub is_default: bool,
    pub sort_order: i32,
}

/// Semua fitur yang dikenal sistem, dalam urutan tampil.
///
/// `max_users` dan `max_members` sengaja berdampingan karena enforcernya
/// berbeda: `max_users` membatasi jumlah baris `tenant_members`
/// (`team_service`), `max_members` hanya tampil sebagai angka limit di
/// halaman subscription. Dulu keduanya tidak saling terkait dan nilainya
/// bisa bertentangan (Pro: users 5, members 10).
pub const FEATURES: &[FeatureDef] = &[
    FeatureDef {
        code: "max_users",
        name: "Maximum Users",
        description: "Maximum number of users allowed (enforced when adding team members)",
        value_type: "number",
        category: "limits",
        default_value: "5",
        sort_order: 0,
    },
    FeatureDef {
        code: "max_members",
        name: "Team Member Limit",
        description: "Maximum number of team members shown on the subscription page",
        value_type: "number",
        category: "general",
        default_value: "2",
        sort_order: 1,
    },
    FeatureDef {
        code: "max_storage_gb",
        name: "Storage (GB)",
        description: "Maximum storage in Gigabytes (decimals allowed, e.g. 0.5)",
        value_type: "number",
        category: "limits",
        default_value: "1",
        sort_order: 2,
    },
    FeatureDef {
        code: "api_access",
        name: "API Access",
        description: "Access to developer API",
        value_type: "boolean",
        category: "capabilities",
        default_value: "false",
        sort_order: 3,
    },
    FeatureDef {
        code: "custom_domain",
        name: "Custom Domain",
        description: "Ability to use custom domain",
        value_type: "boolean",
        category: "branding",
        default_value: "false",
        sort_order: 4,
    },
    FeatureDef {
        code: "remove_branding",
        name: "Remove Branding",
        description: "Remove 'Powered by' branding",
        value_type: "boolean",
        category: "branding",
        default_value: "false",
        sort_order: 5,
    },
    FeatureDef {
        code: "audit_logs",
        name: "Audit Logs",
        description: "Access to audit logs",
        value_type: "boolean",
        category: "security",
        default_value: "false",
        sort_order: 6,
    },
    FeatureDef {
        code: "managed_radius",
        name: "Managed RADIUS",
        description: "Access to managed RADIUS onboarding and centralized PPP authentication",
        value_type: "boolean",
        category: "network",
        default_value: "false",
        sort_order: 7,
    },
    FeatureDef {
        code: "sso_support",
        name: "SSO Support",
        description: "Single Sign-On (SAML/OIDC)",
        value_type: "boolean",
        category: "security",
        default_value: "false",
        sort_order: 8,
    },
    FeatureDef {
        code: "support_level",
        name: "Support Level",
        description: "Level of support (community, standard, priority, dedicated)",
        value_type: "text",
        category: "support",
        default_value: "standard",
        sort_order: 9,
    },
];

/// Plan default. `slug` unik (constraint DB) — seed memakai `ON CONFLICT (slug)`.
pub const PLANS: &[PlanDef] = &[
    PlanDef {
        name: "Free",
        slug: "free",
        description: "Perfect for getting started",
        price_monthly: 0.0,
        price_yearly: 0.0,
        is_active: true,
        is_default: true,
        sort_order: 1,
    },
    PlanDef {
        name: "Pro",
        slug: "pro",
        description: "For growing teams",
        price_monthly: 290_000.0,
        price_yearly: 2_900_000.0,
        is_active: true,
        is_default: false,
        sort_order: 2,
    },
    PlanDef {
        name: "Enterprise",
        slug: "enterprise",
        description: "For large organizations",
        price_monthly: 990_000.0,
        price_yearly: 9_900_000.0,
        is_active: true,
        is_default: false,
        sort_order: 3,
    },
];

/// Nilai fitur per plan (kode fitur, nilai).
///
/// Catatan penting soal `max_storage_gb`: nilainya desimal ("0.5", "50").
/// Pembaca limit (`PlanService::get_feature_limit`) sekarang mem-parse
/// desimal, jadi pecahan tidak lagi diam-diam berubah jadi "unlimited".
///
/// Yang juga penting: SETIAP fitur yang di-enforce wajib punya entri di sini.
/// Dulu `max_users` tidak ada di plan mana pun sehingga ketiga plan jatuh ke
/// `default_value` = 5 — Pro dan Enterprise kena limit 5 user, dan tenant
/// Enterprise dengan 7 anggota langsung diblokir menambah tim.
pub fn plan_feature_values(slug: &str) -> &'static [(&'static str, &'static str)] {
    match slug {
        "free" => &[
            ("max_users", "3"),
            ("max_members", "2"),
            ("max_storage_gb", "1"),
            ("support_level", "community"),
            ("custom_domain", "false"),
            ("api_access", "false"),
            ("audit_logs", "false"),
            ("managed_radius", "false"),
            ("sso_support", "false"),
            ("remove_branding", "false"),
        ],
        "pro" => &[
            ("max_users", "20"),
            ("max_members", "10"),
            ("max_storage_gb", "50"),
            ("support_level", "priority"),
            ("custom_domain", "true"),
            ("api_access", "false"),
            ("audit_logs", "false"),
            ("managed_radius", "false"),
            ("sso_support", "false"),
            ("remove_branding", "false"),
        ],
        "enterprise" => &[
            ("max_users", "unlimited"),
            ("max_members", "unlimited"),
            ("max_storage_gb", "500"),
            ("support_level", "dedicated"),
            ("custom_domain", "true"),
            ("api_access", "true"),
            ("audit_logs", "true"),
            ("managed_radius", "true"),
            ("sso_support", "true"),
            ("remove_branding", "true"),
        ],
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kode_fitur_unik() {
        let mut seen = std::collections::HashSet::new();
        for f in FEATURES {
            assert!(seen.insert(f.code), "kode fitur duplikat: {}", f.code);
        }
    }

    #[test]
    fn slug_plan_unik() {
        let mut seen = std::collections::HashSet::new();
        for p in PLANS {
            assert!(seen.insert(p.slug), "slug plan duplikat: {}", p.slug);
        }
    }

    /// Regresi bug produksi: Pro/Enterprise dulu tidak punya `max_users`
    /// sehingga jatuh ke default 5 dan tenant Enterprise (6-7 anggota)
    /// terblokir menambah tim.
    #[test]
    fn setiap_plan_menetapkan_max_users() {
        for p in PLANS {
            let values = plan_feature_values(p.slug);
            assert!(
                values.iter().any(|(k, _)| *k == "max_users"),
                "plan '{}' tidak menetapkan max_users → akan jatuh ke default dan bisa memblokir tenant",
                p.slug
            );
        }
    }

    /// Setiap nilai per plan harus menunjuk fitur yang benar-benar terdaftar;
    /// entri yatim tidak akan pernah dibaca dan menyembunyikan salah ketik.
    #[test]
    fn nilai_plan_menunjuk_fitur_yang_ada() {
        let known: std::collections::HashSet<&str> = FEATURES.iter().map(|f| f.code).collect();
        for p in PLANS {
            for (code, value) in plan_feature_values(p.slug) {
                assert!(
                    known.contains(code),
                    "plan '{}' menetapkan fitur tak dikenal: {}",
                    p.slug,
                    code
                );
                assert!(
                    !value.trim().is_empty(),
                    "plan '{}' fitur '{}' bernilai kosong — pembaca limit berbeda tafsir (is_truthy=false tapi parse gagal=unlimited)",
                    p.slug,
                    code
                );
            }
        }
    }

    /// `max_storage_gb` boleh desimal; pastikan yang non-"unlimited" bisa
    /// diparse sebagai f64 supaya tidak diam-diam jadi unlimited.
    #[test]
    fn nilai_storage_valid_atau_unlimited() {
        for p in PLANS {
            for (code, value) in plan_feature_values(p.slug) {
                if *code != "max_storage_gb" {
                    continue;
                }
                assert!(
                    value.eq_ignore_ascii_case("unlimited") || value.parse::<f64>().is_ok(),
                    "plan '{}' max_storage_gb tidak bisa diparse: {:?}",
                    p.slug,
                    value
                );
            }
        }
    }

    /// Plan bawaan harus tepat satu supaya factory punya fallback tunggal.
    #[test]
    fn tepat_satu_plan_default() {
        let n = PLANS.iter().filter(|p| p.is_default).count();
        assert_eq!(n, 1, "harus tepat satu plan is_default, dapat {}", n);
    }

    /// Bila Managed RADIUS dibuka untuk Pro, ubah juga test ini — supaya
    /// keputusan produk tidak berubah tanpa sadar.
    #[test]
    fn managed_radius_hanya_enterprise() {
        for p in PLANS {
            let has = plan_feature_values(p.slug)
                .iter()
                .any(|(k, v)| *k == "managed_radius" && v.eq_ignore_ascii_case("true"));
            assert_eq!(
                has,
                p.slug == "enterprise",
                "managed_radius untuk '{}' = {} (diharapkan {})",
                p.slug,
                has,
                p.slug == "enterprise"
            );
        }
    }
}