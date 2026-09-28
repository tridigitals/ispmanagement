//! Team Service for managing tenant members

use crate::db::DbPool;
use crate::models::{TeamMemberWithUser, User};
use crate::services::{AuditService, AuthService, PlanService};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
#[allow(dead_code)]
pub struct TeamService {
    pool: DbPool,
    auth_service: AuthService,
    audit_service: AuditService,
    plan_service: PlanService,
}

impl TeamService {
    pub fn new(
        pool: DbPool,
        auth_service: AuthService,
        audit_service: AuditService,
        plan_service: PlanService,
    ) -> Self {
        Self {
            pool,
            auth_service,
            audit_service,
            plan_service,
        }
    }

    /// List all members of a team
    pub async fn list_members(
        &self,
        tenant_id: &str,
    ) -> Result<Vec<TeamMemberWithUser>, sqlx::Error> {
        #[cfg(feature = "postgres")]
        let query = r#"
            SELECT 
                tm.id, 
                tm.user_id, 
                u.name, 
                u.email, 
                tm.role,
                tm.role_id,
                r.name as role_name,
                u.is_active, 
                tm.created_at,
                -- Owner lama punya role_id NULL, jadi `r.level` (LEFT JOIN)
                -- bernilai NULL dan FE membacanya sebagai level 0 — akibatnya
                -- tombol Edit/Hapus nonaktif untuk SEMUA anggota, termasuk
                -- bagi pemiliknya sendiri. Ambil level dari role global dengan
                -- nama yang sama sebagai cadangan.
                COALESCE(
                    r.level,
                    (SELECT r2.level FROM roles r2
                     WHERE r2.name = tm.role AND r2.tenant_id IS NULL)
                ) as role_level,
                tm.deleted_at,
                u.two_factor_enabled,
                u.email_verified_at
            FROM tenant_members tm
            JOIN users u ON tm.user_id = u.id
            LEFT JOIN roles r ON tm.role_id = r.id
            WHERE tm.tenant_id = $1 AND tm.deleted_at IS NULL
            ORDER BY u.name
        "#;

        #[cfg(feature = "sqlite")]
        let query = r#"
            SELECT 
                tm.id, 
                tm.user_id, 
                u.name, 
                u.email, 
                tm.role,
                tm.role_id,
                r.name as role_name,
                u.is_active, 
                tm.created_at,
                -- Owner lama punya role_id NULL, jadi `r.level` (LEFT JOIN)
                -- bernilai NULL dan FE membacanya sebagai level 0 — akibatnya
                -- tombol Edit/Hapus nonaktif untuk SEMUA anggota, termasuk
                -- bagi pemiliknya sendiri. Ambil level dari role global dengan
                -- nama yang sama sebagai cadangan.
                COALESCE(
                    r.level,
                    (SELECT r2.level FROM roles r2
                     WHERE r2.name = tm.role AND r2.tenant_id IS NULL)
                ) as role_level,
                tm.deleted_at,
                u.two_factor_enabled,
                u.email_verified_at
            FROM tenant_members tm
            JOIN users u ON tm.user_id = u.id
            LEFT JOIN roles r ON tm.role_id = r.id
            WHERE tm.tenant_id = ? AND tm.deleted_at IS NULL
            ORDER BY u.name
        "#;

        sqlx::query_as::<_, TeamMemberWithUser>(query)
            .bind(tenant_id)
            .fetch_all(&self.pool)
            .await
    }

    /// Get user role level
    /// Level role pengguna di sebuah tenant.
    ///
    /// PENTING — fallback Owner. Tenant lama punya `tenant_members.role_id =
    /// NULL` (dibuat sebelum perbaikan `create_tenant`). `JOIN roles` yang
    /// biasa (INNER) tidak menghasilkan baris untuk kasus itu, sehingga fungsi
    /// ini dulu mengembalikan **0** untuk Owner ber-`role_id` NULL.
    ///
    /// Akibatnya nyata dan parah: setiap penambahan anggota tim / penggantian
    /// role membandingkan `requester_level (0) < new_role_level` dan gagal 403
    /// "Permission denied" — bahkan untuk superadmin, dan tanpa cara pulih
    /// lewat UI. `auth_service::has_permission` sudah punya fallback ini;
    /// di sini belum, jadi izin lolos tapi pemeriksaan level menolak.
    ///
    /// Fallback sengaja dibatasi `tm.role IN ('Owner','admin')` supaya bug yang
    /// menulis `role_id = NULL` pada non-owner tidak berubah jadi eskalasi
    /// hak akses (sama seperti penjaga di `auth_service`).
    pub async fn get_user_role_level(&self, user_id: &str, tenant_id: &str) -> Result<i32, String> {
        #[cfg(feature = "postgres")]
        let level: Option<i32> = sqlx::query_scalar("SELECT r.level FROM tenant_members tm JOIN roles r ON tm.role_id = r.id WHERE tm.user_id = $1 AND tm.tenant_id = $2")
            .bind(user_id)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        #[cfg(feature = "postgres")]
        let level: Option<i32> = match level {
            Some(l) => Some(l),
            None => sqlx::query_scalar(
                r#"SELECT r.level FROM tenant_members tm
                   CROSS JOIN roles r
                   WHERE tm.user_id = $1 AND tm.tenant_id = $2
                     AND tm.role_id IS NULL
                     AND tm.role IN ('Owner', 'admin')
                     AND r.name = 'Owner' AND r.tenant_id IS NULL"#,
            )
            .bind(user_id)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?,
        };

        #[cfg(feature = "sqlite")]
        let level: Option<i32> = sqlx::query_scalar("SELECT r.level FROM tenant_members tm JOIN roles r ON tm.role_id = r.id WHERE tm.user_id = ? AND tm.tenant_id = ?")
            .bind(user_id)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        #[cfg(feature = "sqlite")]
        let level: Option<i32> = match level {
            Some(l) => Some(l),
            None => sqlx::query_scalar(
                r#"SELECT r.level FROM tenant_members tm
                   CROSS JOIN roles r
                   WHERE tm.user_id = ? AND tm.tenant_id = ?
                     AND tm.role_id IS NULL
                     AND tm.role IN ('Owner', 'admin')
                     AND r.name = 'Owner' AND r.tenant_id IS NULL"#,
            )
            .bind(user_id)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?,
        };

        Ok(level.unwrap_or(0))
    }

    /// Level role seorang anggota tim.
    ///
    /// PENTING — keamanan. Anggota lama punya `role_id = NULL`, sehingga
    /// `JOIN roles` (INNER) tidak menghasilkan baris dan fungsi ini dulu
    /// mengembalikan **0** untuk Owner. Itu bukan sekadar kosmetik: guard
    /// `enforce_member_role_change_permissions` menolak hanya saat
    /// `requester_level <= target_level`, jadi target yang terbaca 0 bisa
    /// dilewati oleh pemanggil berlevel berapa pun — artinya seorang staf
    /// berlevel rendah dapat mengubah role atau menurunkan Owner.
    ///
    /// Fallback di bawah menyelesaikan level dari kolom teks `tm.role`
    /// (`tenant_members.role` selalu terisi, bahkan saat `role_id` NULL),
    /// dipetakan ke role global dengan nama sama. Hasilnya 0 hanya benar-benar
    /// terjadi bila perannya memang tidak diketahui — dan pemanggil tetap
    /// dilindungi oleh aturan "tidak boleh mengubah anggota setingkat atau di
    /// atasnya".
    pub async fn get_member_role_level(&self, member_id: &str) -> Result<i32, String> {
        #[cfg(feature = "postgres")]
        let level: Option<i32> = sqlx::query_scalar("SELECT r.level FROM tenant_members tm JOIN roles r ON tm.role_id = r.id WHERE tm.id = $1")
            .bind(member_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        #[cfg(feature = "postgres")]
        let level: Option<i32> = match level {
            Some(l) => Some(l),
            None => sqlx::query_scalar(
                r#"SELECT r.level FROM tenant_members tm
                   JOIN roles r ON r.name = tm.role AND r.tenant_id IS NULL
                   WHERE tm.id = $1 AND tm.role_id IS NULL"#,
            )
            .bind(member_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?,
        };

        #[cfg(feature = "sqlite")]
        let level: Option<i32> = sqlx::query_scalar("SELECT r.level FROM tenant_members tm JOIN roles r ON tm.role_id = r.id WHERE tm.id = ?")
            .bind(member_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        #[cfg(feature = "sqlite")]
        let level: Option<i32> = match level {
            Some(l) => Some(l),
            None => sqlx::query_scalar(
                r#"SELECT r.level FROM tenant_members tm
                   JOIN roles r ON r.name = tm.role AND r.tenant_id IS NULL
                   WHERE tm.id = ? AND tm.role_id IS NULL"#,
            )
            .bind(member_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?,
        };

        Ok(level.unwrap_or(0))
    }

    /// Get role level by ID
    pub async fn get_role_level_by_id(&self, role_id: &str) -> Result<i32, String> {
        #[cfg(feature = "postgres")]
        let level: Option<i32> = sqlx::query_scalar("SELECT level FROM roles WHERE id = $1")
            .bind(role_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        #[cfg(feature = "sqlite")]
        let level: Option<i32> = sqlx::query_scalar("SELECT level FROM roles WHERE id = ?")
            .bind(role_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(level.unwrap_or(0))
    }

    /// Get role name by ID — used for validation (e.g. block Customer role in team management)
    pub async fn get_role_name_by_id(&self, role_id: &str) -> Result<Option<String>, String> {
        #[cfg(feature = "postgres")]
        let name: Option<String> = sqlx::query_scalar("SELECT name FROM roles WHERE id = $1")
            .bind(role_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        #[cfg(feature = "sqlite")]
        let name: Option<String> = sqlx::query_scalar("SELECT name FROM roles WHERE id = ?")
            .bind(role_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(name)
    }

    /// Add a new member (create user if needed, or link existing)
    #[allow(clippy::too_many_arguments)]
    pub async fn add_member(
        &self,
        tenant_id: &str,
        email: &str,
        name: &str,
        role_id: &str,
        password: Option<String>,
        is_active: Option<bool>,
        email_verified: Option<bool>,
        actor_id: Option<&str>,
        ip_address: Option<&str>,
    ) -> Result<TeamMemberWithUser, String> {
        // 0. Check Plan Limits (max_users)
        let limit = self
            .plan_service
            .get_feature_limit(tenant_id, "max_users")
            .await
            .map_err(|e| e.to_string())?;

        if let Some(max_users) = limit {
            let count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM tenant_members WHERE tenant_id = $1")
                    .bind(tenant_id)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(|e| e.to_string())?;

            if count >= max_users {
                return Err(format!(
                    "Plan limit reached: Maximum {} users allowed.",
                    max_users
                ));
            }
        }

        // Email dinormalkan SEBELUM apa pun: ia dipakai untuk mencari user
        // lama, dan (bila baru) disimpan. Tanpa ini, "Dodo@gmail.com" membuat
        // baris baru yang berbeda dari "dodo@gmail.com" (UNIQUE Postgres
        // case-sensitive), dan login selalu mencari bentuk lowercase sehingga
        // akun ber-huruf-besar tidak pernah bisa masuk.
        let email = crate::services::email_normalize::normalize_email(email);

        // 1. Check if user exists (case-insensitive: data lama bisa tersimpan
        //    dengan huruf besar).
        let existing_user: Option<User> =
            sqlx::query_as("SELECT * FROM users WHERE lower(email) = lower($1)")
                .bind(&email)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| e.to_string())?;

        let created_user = existing_user.is_none();
        let user_id = if let Some(user) = existing_user {
            // User exists, just link them
            user.id
        } else {
            // User doesn't exist, create them
            let password_str = password.unwrap_or_else(|| Uuid::new_v4().to_string());
            let hash = AuthService::hash_password(&password_str).map_err(|e| e.to_string())?;
            let now = Utc::now();
            let new_id = Uuid::new_v4().to_string();

            // Admin menambahkan akun ini secara manual, jadi statusnya boleh
            // ditentukan di muka: `is_active` (bisa masuk atau tidak) dan
            // `email_verified_at` (dianggap emailnya sudah diverifikasi).
            //
            // Sebelumnya `email_verified_at` TIDAK PERNAH diisi, sehingga akun
            // yang dibuat admin tertinggal dalam keadaan belum terverifikasi.
            // Itu memblokir login begitu tenant menyalakan wajib-verifikasi
            // email (`auth_service`: 400 "email_unverified"), tanpa jalur
            // pemulihan di UI. Default: aktif + terverifikasi, sesuai
            // kenyataan bahwa admin membuatnya sendiri (tidak ada email
            // verifikasi yang pernah dikirim — lihat TODO di bawah).
            let set_active = is_active.unwrap_or(true);
            let verified_at = if email_verified.unwrap_or(true) {
                Some(now)
            } else {
                None
            };

            let query = "INSERT INTO users (id, email, name, password_hash, role, is_active, failed_login_attempts, created_at, updated_at, email_verified_at) VALUES ($1, $2, $3, $4, 'user', $5, 0, $6, $7, $8)";

            #[cfg(feature = "postgres")]
            sqlx::query(query)
                .bind(&new_id)
                .bind(&email)
                .bind(name)
                .bind(hash)
                .bind(set_active)
                .bind(now)
                .bind(now)
                .bind(verified_at)
                .execute(&self.pool)
                .await
                .map_err(|e| e.to_string())?;

            #[cfg(feature = "sqlite")]
            sqlx::query(query)
                .bind(&new_id)
                .bind(&email)
                .bind(name)
                .bind(hash)
                .bind(set_active)
                .bind(now.to_rfc3339())
                .bind(now.to_rfc3339())
                .bind(verified_at.map(|t| t.to_rfc3339()))
                .execute(&self.pool)
                .await
                .map_err(|e| e.to_string())?;

            // TODO: Send welcome email with password reset link

            new_id
        };

        // 2. Get role details
        let role_name: String = sqlx::query_scalar("SELECT name FROM roles WHERE id = $1")
            .bind(role_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| "Role not found".to_string())?;

        // 3. Add to tenant_members.
        //
        // `remove_member` melakukan SOFT DELETE (hanya mengisi `deleted_at`),
        // sedangkan baris `users` sengaja dipertahankan. Jadi email yang pernah
        // dihapus masih ada di tabel `users` (unique) DAN baris lamanya masih
        // ada di `tenant_members` dengan `deleted_at` terisi.
        //
        // Dulu pemeriksaannya `EXISTS(... user_id = $2)` tanpa memandang
        // `deleted_at`, sehingga menambahkan kembali email tersebut selalu
        // ditolak. Karena seluruh kegagalan service dikembalikan `Err(String)`
        // dan dipetakan ke HTTP 500, pengguna melihat "500 Internal server
        // error" — bukan pesan yang bisa dimengerti.
        //
        // Perilaku yang benar: mengaktifkan kembali keanggotaan lama (pulihkan
        // `deleted_at`, set ulang role) alih-alih menolak.
        #[cfg(feature = "postgres")]
        let existing_member: Option<(String, Option<String>)> = sqlx::query_as(
            "SELECT id, deleted_at::text FROM tenant_members WHERE tenant_id = $1 AND user_id = $2 ORDER BY created_at LIMIT 1",
        )
        .bind(tenant_id)
        .bind(&user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        #[cfg(feature = "sqlite")]
        let existing_member: Option<(String, Option<String>)> = sqlx::query_as(
            "SELECT id, deleted_at FROM tenant_members WHERE tenant_id = ? AND user_id = ? ORDER BY created_at LIMIT 1",
        )
        .bind(tenant_id)
        .bind(&user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        let now = Utc::now();

        let member_id = match existing_member {
            // Sudah anggota aktif -> tolak dengan pesan yang jelas.
            Some((_, ref deleted)) if deleted.is_none() => {
                return Err("User is already a member of this team".to_string());
            }
            // Pernah dihapus -> pulihkan keanggotaan lama.
            Some((id, _)) => {
                #[cfg(feature = "postgres")]
                sqlx::query(
                    "UPDATE tenant_members SET deleted_at = NULL, role = $1, role_id = $2 WHERE id = $3",
                )
                .bind(&role_name)
                .bind(role_id)
                .bind(&id)
                .execute(&self.pool)
                .await
                .map_err(|e| e.to_string())?;

                #[cfg(feature = "sqlite")]
                sqlx::query(
                    "UPDATE tenant_members SET deleted_at = NULL, role = ?, role_id = ? WHERE id = ?",
                )
                .bind(&role_name)
                .bind(role_id)
                .bind(&id)
                .execute(&self.pool)
                .await
                .map_err(|e| e.to_string())?;

                id
            }
            // Belum pernah jadi anggota -> buat baris baru.
            None => {
                let id = Uuid::new_v4().to_string();

                let query = "INSERT INTO tenant_members (id, tenant_id, user_id, role, role_id, created_at) VALUES ($1, $2, $3, $4, $5, $6)";

                #[cfg(feature = "postgres")]
                sqlx::query(query)
                    .bind(&id)
                    .bind(tenant_id)
                    .bind(&user_id)
                    .bind(&role_name) // Fallback string role
                    .bind(role_id)
                    .bind(now)
                    .execute(&self.pool)
                    .await
                    .map_err(|e| e.to_string())?;

                #[cfg(feature = "sqlite")]
                sqlx::query(query)
                    .bind(&id)
                    .bind(tenant_id)
                    .bind(&user_id)
                    .bind(&role_name)
                    .bind(role_id)
                    .bind(now.to_rfc3339())
                    .execute(&self.pool)
                    .await
                    .map_err(|e| e.to_string())?;

                id
            }
        };

        // Audit Log
        let details = serde_json::json!({
            "message": "Added team member",
            "tenant_id": tenant_id,
            "member_id": member_id,
            "user_id": user_id,
            "email": email,
            "name": name,
            "role_name": role_name,
            "role_id": role_id,
            "created_user": created_user
        })
        .to_string();
        self.audit_service
            .log(
                actor_id,
                Some(tenant_id),
                "TEAM_MEMBER_ADD",
                "team",
                Some(&user_id),
                Some(details.as_str()),
                ip_address,
            )
            .await;

        // Return the created member.
        //
        // Status dibaca dari DB, bukan di-hardcode. Sebelumnya selalu
        // `is_active: true` / `email_verified_at: None` walau pemanggil meminta
        // akun nonaktif atau sudah terverifikasi — jadi tabel di UI tidak
        // mencerminkan apa yang baru saja disimpan.
        #[cfg(feature = "postgres")]
        let status: (bool, Option<chrono::DateTime<Utc>>) = sqlx::query_as(
            "SELECT is_active, email_verified_at FROM users WHERE id = $1",
        )
        .bind(&user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        #[cfg(feature = "sqlite")]
        let status: (bool, Option<chrono::DateTime<Utc>>) = {
            let raw: (bool, Option<String>) =
                sqlx::query_as("SELECT is_active, email_verified_at FROM users WHERE id = ?")
                    .bind(&user_id)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(|e| e.to_string())?;
            (
                raw.0,
                raw.1.and_then(|s| {
                    chrono::DateTime::parse_from_rfc3339(&s)
                        .ok()
                        .map(|d| d.with_timezone(&Utc))
                }),
            )
        };

        Ok(TeamMemberWithUser {
            id: member_id,
            user_id,
            name: name.to_string(),
            email: email.to_string(),
            role: role_name.clone(),
            role_id: Some(role_id.to_string()),
            role_name: Some(role_name),
            is_active: status.0,
            created_at: now,
            role_level: None,
            // Keanggotaan yang dipulihkan mempertahankan `created_at` lama; ini
            // hanya memengaruhi tampilan "bergabung sejak".
            deleted_at: None,
            two_factor_enabled: Some(false),
            email_verified_at: status.1,
        })
    }

    /// Update member role
    /// Perbarui anggota tim. Ketiga parameter bersifat opsional dan
    /// independen, sehingga form edit yang sama bisa dipakai untuk mengganti
    /// role saja, mengaktifkan/menonaktifkan akun, atau menandai email sudah
    /// diverifikasi — tanpa memaksa memuat ulang field lain.
    ///
    /// Catatan: `role_id` dulu wajib. Sekarang opsional karena mengubah status
    /// akun saja tidak boleh menuntut perubahan role.
    pub async fn update_member(
        &self,
        tenant_id: &str,
        member_id: &str,
        role_id: Option<&str>,
        is_active: Option<bool>,
        email_verified: Option<bool>,
        actor_id: Option<&str>,
        ip_address: Option<&str>,
    ) -> Result<(), String> {
        #[cfg(feature = "postgres")]
        let before: Option<(String, String, Option<String>, Option<String>)> = sqlx::query_as(
            r#"
            SELECT tm.user_id::text, u.email, tm.role, tm.role_id::text
            FROM tenant_members tm
            JOIN users u ON u.id = tm.user_id
            WHERE tm.id = $1 AND tm.tenant_id = $2
        "#,
        )
        .bind(member_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        #[cfg(feature = "sqlite")]
        let before: Option<(String, String, Option<String>, Option<String>)> = sqlx::query_as(
            r#"
            SELECT tm.user_id, u.email, tm.role, tm.role_id
            FROM tenant_members tm
            JOIN users u ON u.id = tm.user_id
            WHERE tm.id = ? AND tm.tenant_id = ?
        "#,
        )
        .bind(member_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        let (user_id, email, role_name_before, role_id_before) =
            before.ok_or_else(|| "Member not found".to_string())?;

        // Hanya sentuh tenant_members bila memang ada pergantian role.
        let role_change: Option<(String, String)> = match role_id {
            Some(rid) => {
                let name: String = sqlx::query_scalar("SELECT name FROM roles WHERE id = $1")
                    .bind(rid)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(|_| "Role not found".to_string())?;
                Some((name, rid.to_string()))
            }
            None => None,
        };

        if let Some((role_name, rid)) = role_change.as_ref() {
            sqlx::query("UPDATE tenant_members SET role = $1, role_id = $2 WHERE id = $3")
                .bind(role_name)
                .bind(rid)
                .bind(member_id)
                .execute(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
        }

        // is_active dan email_verified_at hidup di tabel `users`, bukan
        // `tenant_members`. FK sudah diverifikasi lewat JOIN di atas, jadi
        // aman memakai user_id hasil lookup tersebut.
        if is_active.is_some() || email_verified.is_some() {
            let now = Utc::now();
            if let Some(active) = is_active {
                sqlx::query("UPDATE users SET is_active = $1, updated_at = $2 WHERE id = $3")
                    .bind(active)
                    .bind(now)
                    .bind(&user_id)
                    .execute(&self.pool)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            if let Some(verified) = email_verified {
                // Menandai belum terverifikasi juga menghapus jejak waktunya,
                // supaya kolom tidak menyimpan tanggal yang menyesatkan.
                let at = if verified { Some(now) } else { None };
                sqlx::query(
                    "UPDATE users SET email_verified_at = $1, updated_at = $2 WHERE id = $3",
                )
                .bind(at)
                .bind(now)
                .bind(&user_id)
                .execute(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
            }
        }

        // Audit Log
        let details = serde_json::json!({
            "message": "Updated team member",
            "tenant_id": tenant_id,
            "member_id": member_id,
            "user_id": user_id,
            "email": email,
            "role_name_before": role_name_before,
            "role_id_before": role_id_before,
            "role_name_after": role_change.as_ref().map(|(n, _)| n.clone()),
            "role_id_after": role_change.as_ref().map(|(_, id)| id.clone()),
            "is_active": is_active,
            "email_verified": email_verified
        })
        .to_string();
        self.audit_service
            .log(
                actor_id,
                Some(tenant_id),
                "TEAM_MEMBER_UPDATE",
                "team",
                Some(&user_id),
                Some(details.as_str()),
                ip_address,
            )
            .await;

        Ok(())
    }

    /// Remove member (soft delete)
    pub async fn remove_member(
        &self,
        tenant_id: &str,
        member_id: &str,
        actor_id: Option<&str>,
        ip_address: Option<&str>,
    ) -> Result<(), String> {
        #[cfg(feature = "postgres")]
        let before: Option<(String, String, Option<String>, Option<String>)> = sqlx::query_as(
            r#"
            SELECT tm.user_id::text, u.email, tm.role, tm.role_id::text
            FROM tenant_members tm
            JOIN users u ON u.id = tm.user_id
            WHERE tm.id = $1 AND tm.tenant_id = $2 AND tm.deleted_at IS NULL
        "#,
        )
        .bind(member_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        #[cfg(feature = "sqlite")]
        let before: Option<(String, String, Option<String>, Option<String>)> = sqlx::query_as(
            r#"
            SELECT tm.user_id, u.email, tm.role, tm.role_id
            FROM tenant_members tm
            JOIN users u ON u.id = tm.user_id
            WHERE tm.id = ? AND tm.tenant_id = ? AND tm.deleted_at IS NULL
        "#,
        )
        .bind(member_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        let (user_id, email, role_name_before, role_id_before) =
            before.ok_or_else(|| "Member not found or already deleted".to_string())?;

        #[cfg(feature = "postgres")]
        sqlx::query("UPDATE tenant_members SET deleted_at = NOW() WHERE id = $1")
            .bind(member_id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        #[cfg(feature = "sqlite")]
        sqlx::query("UPDATE tenant_members SET deleted_at = datetime('now') WHERE id = ?")
            .bind(member_id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        // Audit Log
        let details = serde_json::json!({
            "message": "Removed team member",
            "tenant_id": tenant_id,
            "member_id": member_id,
            "user_id": user_id,
            "email": email,
            "role_name_before": role_name_before,
            "role_id_before": role_id_before
        })
        .to_string();
        self.audit_service
            .log(
                actor_id,
                Some(tenant_id),
                "TEAM_MEMBER_REMOVE",
                "team",
                Some(&user_id),
                Some(details.as_str()),
                ip_address,
            )
            .await;

        Ok(())
    }

    /// List deleted members of a team
    pub async fn list_deleted_members(
        &self,
        tenant_id: &str,
    ) -> Result<Vec<TeamMemberWithUser>, sqlx::Error> {
        #[cfg(feature = "postgres")]
        let query = r#"
            SELECT 
                tm.id, 
                tm.user_id, 
                u.name, 
                u.email, 
                tm.role,
                tm.role_id,
                r.name as role_name,
                u.is_active, 
                tm.created_at,
                r.level as role_level,
                tm.deleted_at,
                u.two_factor_enabled,
                u.email_verified_at
            FROM tenant_members tm
            JOIN users u ON tm.user_id = u.id
            LEFT JOIN roles r ON tm.role_id = r.id
            WHERE tm.tenant_id = $1 AND tm.deleted_at IS NOT NULL
            ORDER BY tm.deleted_at DESC
        "#;

        #[cfg(feature = "sqlite")]
        let query = r#"
            SELECT 
                tm.id, 
                tm.user_id, 
                u.name, 
                u.email, 
                tm.role,
                tm.role_id,
                r.name as role_name,
                u.is_active, 
                tm.created_at,
                r.level as role_level,
                tm.deleted_at,
                u.two_factor_enabled,
                u.email_verified_at
            FROM tenant_members tm
            JOIN users u ON tm.user_id = u.id
            LEFT JOIN roles r ON tm.role_id = r.id
            WHERE tm.tenant_id = ? AND tm.deleted_at IS NOT NULL
            ORDER BY tm.deleted_at DESC
        "#;

        sqlx::query_as::<_, TeamMemberWithUser>(query)
            .bind(tenant_id)
            .fetch_all(&self.pool)
            .await
    }

    /// Restore a soft-deleted member
    pub async fn restore_member(
        &self,
        tenant_id: &str,
        member_id: &str,
        actor_id: Option<&str>,
        ip_address: Option<&str>,
    ) -> Result<(), String> {
        #[cfg(feature = "postgres")]
        let before: Option<(String, String)> = sqlx::query_as(
            r#"SELECT tm.user_id::text, u.email FROM tenant_members tm JOIN users u ON u.id = tm.user_id WHERE tm.id = $1 AND tm.tenant_id = $2 AND tm.deleted_at IS NOT NULL"#,
        )
        .bind(member_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        #[cfg(feature = "sqlite")]
        let before: Option<(String, String)> = sqlx::query_as(
            r#"SELECT tm.user_id, u.email FROM tenant_members tm JOIN users u ON u.id = tm.user_id WHERE tm.id = ? AND tm.tenant_id = ? AND tm.deleted_at IS NOT NULL"#,
        )
        .bind(member_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        let (user_id, email) = before.ok_or_else(|| "Deleted member not found".to_string())?;

        sqlx::query("UPDATE tenant_members SET deleted_at = NULL WHERE id = $1")
            .bind(member_id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        let details = serde_json::json!({
            "message": "Restored team member",
            "tenant_id": tenant_id,
            "member_id": member_id,
            "user_id": user_id,
            "email": email
        })
        .to_string();
        self.audit_service
            .log(
                actor_id,
                Some(tenant_id),
                "TEAM_MEMBER_RESTORE",
                "team",
                Some(&user_id),
                Some(details.as_str()),
                ip_address,
            )
            .await;

        Ok(())
    }

    /// Permanently delete a soft-deleted member (and orphan user if applicable)
    pub async fn hard_delete_member(
        &self,
        tenant_id: &str,
        member_id: &str,
        actor_id: Option<&str>,
        ip_address: Option<&str>,
    ) -> Result<(), String> {
        let user_id: String = sqlx::query_scalar(
            "SELECT user_id FROM tenant_members WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL",
        )
        .bind(member_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Deleted member not found")?;

        sqlx::query("DELETE FROM tenant_members WHERE id = $1")
            .bind(member_id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        // Cleanup orphan user
        let has_other_memberships: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tenant_members WHERE user_id = $1)")
                .bind(&user_id)
                .fetch_one(&self.pool)
                .await
                .unwrap_or(false);

        let has_customer_links: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM customer_users WHERE user_id = $1)")
                .bind(&user_id)
                .fetch_one(&self.pool)
                .await
                .unwrap_or(false);

        if !has_other_memberships && !has_customer_links {
            sqlx::query("DELETE FROM users WHERE id = $1")
                .bind(&user_id)
                .execute(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
        }

        let details = serde_json::json!({
            "message": "Permanently deleted team member",
            "tenant_id": tenant_id,
            "member_id": member_id,
            "user_id": user_id
        })
        .to_string();
        self.audit_service
            .log(
                actor_id,
                Some(tenant_id),
                "TEAM_MEMBER_HARD_DELETE",
                "team",
                None,
                Some(details.as_str()),
                ip_address,
            )
            .await;

        Ok(())
    }
}
