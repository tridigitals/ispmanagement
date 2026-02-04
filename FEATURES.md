# 🚀 RustTauri SaaS Boilerplate - Dokumentasi Fitur

> **Last Updated:** Januari 2026  
> **Stack:** Rust (Tauri) + SvelteKit + SQLite/PostgreSQL

---

## 📋 Ringkasan Fitur

| Kategori                  | Jumlah Fitur | Status      |
| ------------------------- | ------------ | ----------- |
| Authentication & Security | 15+          | ✅ Complete |
| Multi-Tenancy             | 8            | ✅ Complete |
| Authorization (RBAC)      | 5            | ✅ Complete |
| Billing & Payment         | 8            | ✅ Complete |
| Notifications             | 6            | ✅ Complete |
| Storage                   | 7            | ✅ Complete |
| Email                     | 5            | ✅ Complete |
| System & Monitoring       | 5            | ✅ Complete |
| Frontend Components       | 16+          | ✅ Complete |
| Internationalization      | 2 languages  | ✅ Complete |

---

## 🔐 Authentication & Security

### Password & Login

| Fitur                  | Deskripsi                                      | File Terkait      |
| ---------------------- | ---------------------------------------------- | ----------------- |
| JWT Token              | Access + Refresh token dengan expiry           | `auth_service.rs` |
| Argon2 Hashing         | Password hashing modern yang aman              | `auth_service.rs` |
| Password Policy        | Min length, special chars, uppercase, numbers  | `auth_service.rs` |
| Brute Force Protection | Account lockout setelah N gagal login          | `auth_service.rs` |
| Session Management     | Track active sessions, logout from all devices | `auth_service.rs` |

### Two-Factor Authentication (2FA)

| Fitur                        | Deskripsi                                        | File Terkait      |
| ---------------------------- | ------------------------------------------------ | ----------------- |
| TOTP                         | Time-based OTP (Google Authenticator compatible) | `auth_service.rs` |
| Email OTP                    | One-time password via email                      | `auth_service.rs` |
| 2FA Setup Flow               | QR code generation, backup codes                 | `auth_service.rs` |
| Tenant-level 2FA Enforcement | Force 2FA untuk semua user di tenant             | `tenant.rs`       |

### Account Management

| Fitur               | Deskripsi                        | File Terkait      |
| ------------------- | -------------------------------- | ----------------- |
| Email Verification  | Verifikasi email saat registrasi | `auth_service.rs` |
| Forgot Password     | Reset password via email         | `auth_service.rs` |
| Installation Wizard | Setup admin pertama kali         | `install.rs`      |

---

## 🏢 Multi-Tenancy

| Fitur                    | Deskripsi                               | File Terkait          |
| ------------------------ | --------------------------------------- | --------------------- |
| Tenant Isolation         | Data terpisah per tenant                | `tenant.rs`           |
| Custom Domain            | Setiap tenant bisa punya domain sendiri | `tenant.rs`           |
| Tenant Slug Routing      | URL: `/[tenant]/dashboard`              | `src/routes/[tenant]` |
| Tenant-specific Settings | Settings berbeda per tenant             | `settings_service.rs` |
| Tenant Logo              | Custom logo per tenant                  | `tenant.rs`           |
| Tenant Active/Inactive   | Enable/disable tenant                   | `tenant.rs`           |
| Tenant Members           | Daftar anggota dengan role              | `team_service.rs`     |
| Multi-tenant User        | Satu user bisa di banyak tenant         | `user.rs`             |

---

## 👥 Authorization (RBAC)

| Fitur                  | Deskripsi                            | File Terkait      |
| ---------------------- | ------------------------------------ | ----------------- |
| Roles                  | Admin, User, + custom roles          | `role_service.rs` |
| Permissions            | Granular permissions per resource    | `role_service.rs` |
| Role Assignment        | Assign role ke user                  | `role_service.rs` |
| System vs Custom Roles | Default roles + tenant-created roles | `role_service.rs` |
| Role Hierarchy         | Level-based role comparison          | `team_service.rs` |

### Default Permissions

```
users:read, users:write, users:delete
roles:read, roles:write, roles:delete
settings:read, settings:write
team:read, team:write, team:delete
storage:read, storage:write, storage:delete
billing:read, billing:write
```

---

## 💳 Billing & Payment

### Payment Gateway

| Fitur                | Deskripsi                        | File Terkait         |
| -------------------- | -------------------------------- | -------------------- |
| Midtrans Integration | Snap token, webhook notification | `payment_service.rs` |
| Manual Bank Transfer | Upload bukti transfer            | `payment_service.rs` |
| Payment Verification | Admin approve/reject pembayaran  | `payment_service.rs` |

### Invoice Management

| Fitur           | Deskripsi                               | File Terkait         |
| --------------- | --------------------------------------- | -------------------- |
| Create Invoice  | Generate invoice dengan nomor unik      | `payment_service.rs` |
| Invoice Listing | List per tenant atau semua (SuperAdmin) | `payment_service.rs` |
| Payment Status  | pending → paid/failed/cancelled         | `payment_service.rs` |
| Payment Page    | Public page `/pay/[id]`                 | `src/routes/pay`     |

### Subscription & Plans

| Fitur                | Deskripsi                      | File Terkait      |
| -------------------- | ------------------------------ | ----------------- |
| Plans Management     | Create/edit/delete plans       | `plan_service.rs` |
| Feature Definitions  | Boolean/Text/Number features   | `plan_service.rs` |
| Plan Features        | Assign features to plans       | `plan_service.rs` |
| Tenant Subscription  | Assign plan ke tenant          | `plan_service.rs` |
| Auto-Expiration      | Downgrade ke Free saat expired | `plan_service.rs` |
| Feature Access Check | Check tenant feature access    | `plan_service.rs` |
| Billing Cycle        | Monthly/Yearly pricing         | `plan_service.rs` |

### Bank Accounts (Admin)

| Fitur                  | Deskripsi                      | File Terkait         |
| ---------------------- | ------------------------------ | -------------------- |
| Bank Account CRUD      | Manage rekening untuk transfer | `payment_service.rs` |
| Active/Inactive Toggle | Enable/disable rekening        | `payment_service.rs` |

---

## 🔔 Notifications

| Fitur                    | Deskripsi                              | File Terkait                              |
| ------------------------ | -------------------------------------- | ----------------------------------------- |
| Real-time WebSocket      | Push notifikasi instant                | `websocket.rs`, `notification_service.rs` |
| In-app Notifications     | Bell dropdown dengan list              | `NotificationDropdown.svelte`             |
| Web Push                 | Browser push notifications (PWA ready) | `notification_service.rs`                 |
| Email Notifications      | Kirim notifikasi via email             | `notification_service.rs`                 |
| Notification Preferences | User bisa atur channel per kategori    | `notification_service.rs`                 |
| Mark Read/Unread         | Mark as read, mark all as read         | `notification_service.rs`                 |

---

## 📁 Storage

| Fitur                 | Deskripsi                                          | File Terkait         |
| --------------------- | -------------------------------------------------- | -------------------- |
| Local Storage         | Simpan file di server                              | `storage_service.rs` |
| S3 Compatible         | AWS S3, DigitalOcean Spaces, MinIO                 | `storage_service.rs` |
| Chunked Upload        | Upload file besar per chunk                        | `storage_service.rs` |
| File Manager UI       | Browse, upload, delete files                       | `FileManager.svelte` |
| Tenant Storage Quota  | Limit storage per plan                             | `storage_service.rs` |
| File Metadata         | Track original name, size, type                    | `file.rs`            |
| Admin vs Tenant Files | SuperAdmin lihat semua, tenant lihat milik sendiri | `storage_service.rs` |

---

## 📧 Email

| Fitur          | Deskripsi                      | File Terkait       |
| -------------- | ------------------------------ | ------------------ |
| SMTP Support   | Send via SMTP server           | `email_service.rs` |
| Resend API     | Send via Resend                | `email_service.rs` |
| SendGrid API   | Send via SendGrid              | `email_service.rs` |
| Custom Webhook | Send email via custom endpoint | `email_service.rs` |
| Test Email     | Kirim test email dari settings | `email_service.rs` |

---

## 🗄️ Database

| Fitur                | Deskripsi                       | File Terkait    |
| -------------------- | ------------------------------- | --------------- |
| SQLite Support       | Development & small deployments | `connection.rs` |
| PostgreSQL Support   | Production-grade database       | `connection.rs` |
| UUID Primary Keys    | Semua table pakai UUID          | all models      |
| Automatic Migrations | Schema seeding saat startup     | `lib.rs`        |

---

## 📊 System & Monitoring

| Fitur            | Deskripsi                      | File Terkait                       |
| ---------------- | ------------------------------ | ---------------------------------- |
| Audit Logging    | Log semua aksi user            | `audit_service.rs`                 |
| Audit Log Viewer | UI untuk browse audit logs     | `src/routes/superadmin/audit-logs` |
| System Health    | CPU, Memory, Disk usage        | `system_service.rs`                |
| Database Stats   | Table count, size, connections | `system_service.rs`                |
| Recent Activity  | Latest actions in system       | `system_service.rs`                |

---

## 🎨 Frontend Components

| Component                     | Deskripsi                        |
| ----------------------------- | -------------------------------- |
| `Table.svelte`                | Sortable table dengan pagination |
| `Pagination.svelte`           | Page navigation                  |
| `Modal.svelte`                | Dialog modal                     |
| `ConfirmDialog.svelte`        | Confirm yes/no dialog            |
| `FileManager.svelte`          | File browser dengan upload       |
| `Lightbox.svelte`             | Image/video preview              |
| `Sidebar.svelte`              | Navigation sidebar               |
| `Topbar.svelte`               | Top navigation bar               |
| `NotificationDropdown.svelte` | Bell notification dropdown       |
| `StatsCard.svelte`            | Dashboard statistics card        |
| `Icon.svelte`                 | SVG icon wrapper                 |
| `Input.svelte`                | Form input component             |
| `Select.svelte`               | Dropdown select                  |
| `TableToolbar.svelte`         | Table actions toolbar            |
| `MobileFabMenu.svelte`        | Mobile floating action button    |
| `GlobalUploads.svelte`        | Upload progress indicator        |

---

## 🌐 Internationalization (i18n)

| Bahasa           | File              |
| ---------------- | ----------------- |
| English          | `locales/en.json` |
| Bahasa Indonesia | `locales/id.json` |

---

## 📱 Pages & Routes

### Public Pages

| Route              | Deskripsi                      |
| ------------------ | ------------------------------ |
| `/`                | Landing page                   |
| `/login`           | Login page                     |
| `/register`        | Registration page              |
| `/forgot-password` | Password recovery              |
| `/verify-email`    | Email verification             |
| `/install`         | First-time installation wizard |
| `/pay/[id]`        | Public payment page            |
| `/maintenance`     | Maintenance mode page          |
| `/unauthorized`    | Access denied page             |

### Tenant Pages (`/[tenant]/...`)

| Route                          | Deskripsi               |
| ------------------------------ | ----------------------- |
| `/[tenant]/dashboard`          | Tenant dashboard        |
| `/[tenant]/profile`            | User profile            |
| `/[tenant]/storage`            | File manager            |
| `/[tenant]/admin`              | Admin dashboard         |
| `/[tenant]/admin/team`         | Team/member management  |
| `/[tenant]/admin/roles`        | Role management         |
| `/[tenant]/admin/settings`     | Tenant settings         |
| `/[tenant]/admin/invoices`     | Invoice list            |
| `/[tenant]/admin/subscription` | Subscription management |
| `/[tenant]/admin/storage`      | Admin file manager      |

### SuperAdmin Pages (`/superadmin/...`)

| Route                    | Deskripsi              |
| ------------------------ | ---------------------- |
| `/superadmin`            | SuperAdmin dashboard   |
| `/superadmin/tenants`    | Tenant management      |
| `/superadmin/users`      | Global user management |
| `/superadmin/plans`      | Plan management        |
| `/superadmin/invoices`   | All invoices           |
| `/superadmin/settings`   | Global settings        |
| `/superadmin/storage`    | Global file manager    |
| `/superadmin/audit-logs` | Audit log viewer       |
| `/superadmin/system`     | System health          |

---

## ⚙️ Settings Categories

| Category    | Settings                                                                                                                                              |
| ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| **General** | app_name, app_description, maintenance_mode                                                                                                           |
| **Auth**    | session_timeout, max_login_attempts, lockout_duration, require_uppercase, require_number, require_special, min_password_length                        |
| **Email**   | email_provider, smtp_host, smtp_port, smtp_user, smtp_password, smtp_encryption, resend_api_key, sendgrid_api_key, webhook_url, from_email, from_name |
| **Storage** | storage_provider, s3_bucket, s3_region, s3_access_key, s3_secret_key, s3_endpoint                                                                     |
| **Payment** | midtrans_server_key, midtrans_client_key, midtrans_is_production                                                                                      |
| **Push**    | vapid_public_key, vapid_private_key, vapid_subject                                                                                                    |

---

## 🖥️ HTTP Server

| Fitur          | Deskripsi                         |
| -------------- | --------------------------------- |
| Axum Framework | Rust async web framework          |
| Dynamic CORS   | Allow origins dari database       |
| WebSocket Hub  | Real-time communication           |
| Body Limit     | 50MB default, 500MB untuk storage |
| Static Files   | Serve uploaded files              |

---

## 📦 Tech Stack Summary

### Backend

- **Runtime:** Rust + Tauri
- **Web Framework:** Axum
- **Database:** SQLite / PostgreSQL (sqlx)
- **Auth:** jsonwebtoken, argon2, totp-rs
- **Email:** lettre (SMTP), reqwest (API)
- **Storage:** Local filesystem, AWS S3
- **Push:** web-push-native

### Frontend

- **Framework:** SvelteKit
- **API Client:** Custom TypeScript client
- **State:** Svelte stores
- **i18n:** Custom implementation
- **Styling:** CSS

---

_Dokumentasi ini di-generate berdasarkan analisis codebase pada Januari 2026._
