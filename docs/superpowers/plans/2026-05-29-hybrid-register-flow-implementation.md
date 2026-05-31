# Hybrid Register Flow — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use [executing-plans] mode to implement this plan task-by-task.

**Goal:** Implement a hybrid public registration flow where new users land in a `pending` state, receive no tenant attachment, cannot login until reviewed, and superadmin approves/rejects via an admin panel.

**Architecture:** Public `/api/auth/register` creates users with `registration_status='pending'` instead of `active`. A new `approve_pending_user` / `reject_pending_user` service layer in `auth_service` handles the lifecycle. Superadmin-only endpoints gated by `claims.is_super_admin` expose list/approve/reject. Email notification to superadmin uses existing `email_outbox` pattern. Frontend register page shows pending confirmation; login page intercepts `AccountPendingApproval`; new superadmin page lists pending users.

**Tech Stack:** Rust, SQLx, Axum, Tauri, SvelteKit, TypeScript, svelte-i18n

---

## 1. Context & Goal

### Current State (BLOCKER #2 from MVP DoD Audit)

In [`src-tauri/src/services/auth_service/mod.rs:569`](src-tauri/src/services/auth_service/mod.rs:569), `register()` inserts into `users` with **no tenant attachment**. When `allow_registration` is `true` (toggle at [`src-tauri/src/services/auth_service/dto.rs:65`](src-tauri/src/services/auth_service/dto.rs:65)), the newly created user gets `claims.tenant_id = None`. This causes every handler using the `tenant_and_claims` pattern (found in 20+ files under [`src-tauri/src/http/`](src-tauri/src/http/)) to return `Unauthorized`. The user can register but is immediately locked out of every tenant-scoped route — a critical UX and security gap.

### Desired State

1. Public `/register` creates a user with `registration_status='pending'`, **no token issued**.
2. Superadmin receives email notification (via `email_outbox`) that a new user is pending.
3. Superadmin reviews via **Admin Panel → Pending Approvals**: approve (assign to tenant + role) or reject (mark rejected with reason).
4. Pending users **cannot login** — login returns a clear `AccountPendingApproval` error code the frontend displays.
5. Approved users can login normally with their assigned tenant and role.

### Audit Finding Reference

- BLOCKER #2: "Public registration creates orphaned users with no tenant, making all tenant-scoped routes inaccessible."
- Severity: Blocker — blocks any tenant-scoped feature for publicly registered users.

---

## 2. Non-Goals

- **No rewrite of `auth_service`** — we extend existing `register()` and `login()` methods.
- **No changes to `customer_service/registration.rs` invite-token flow** — the tenant-scoped invite-token flow is separate and remains untouched.
- **No multi-tenant pending approval** — pending users are global (no tenant_id). Tenant-scoped invite flow already exists separately.
- **No soft-delete expiry / retention policy for rejected users** — out of scope for this MVP fix; can be added later.
- **No changes to existing superadmin user management pages** — only adding a new "Pending Approvals" section.
- **No RBAC role-permission matrix changes** beyond adding one new permission entry for registration approvals.

---

## 3. Schema Changes

### Migration Files

- **Create:** `src-tauri/migrations/20260529130000_add_user_registration_status.up.sql`
- **Create:** `src-tauri/migrations/20260529130000_add_user_registration_status.down.sql`

### UP Migration

```sql
-- Add registration_status and audit columns to users table.
-- Existing users default to 'active' so no data migration needed.

ALTER TABLE public.users
    ADD COLUMN registration_status VARCHAR(20) NOT NULL DEFAULT 'active'
        CHECK (registration_status IN ('active', 'pending', 'rejected'));

ALTER TABLE public.users
    ADD COLUMN pending_review_message TEXT;

ALTER TABLE public.users
    ADD COLUMN approved_at TIMESTAMPTZ;

ALTER TABLE public.users
    ADD COLUMN approved_by_user_id TEXT;

ALTER TABLE public.users
    ADD COLUMN rejected_at TIMESTAMPTZ;

ALTER TABLE public.users
    ADD COLUMN rejected_reason TEXT;

ALTER TABLE public.users
    ADD COLUMN rejected_by_user_id TEXT;

-- Index for fast filtering of pending users (superadmin list query)
CREATE INDEX idx_users_registration_status ON public.users (registration_status)
    WHERE registration_status != 'active';

-- FK constraints (text-based, matching existing id pattern)
ALTER TABLE public.users
    ADD CONSTRAINT fk_users_approved_by
    FOREIGN KEY (approved_by_user_id) REFERENCES public.users(id) ON DELETE SET NULL;

ALTER TABLE public.users
    ADD CONSTRAINT fk_users_rejected_by
    FOREIGN KEY (rejected_by_user_id) REFERENCES public.users(id) ON DELETE SET NULL;
```

### DOWN Migration

```sql
DROP INDEX IF EXISTS idx_users_registration_status;

ALTER TABLE public.users DROP CONSTRAINT IF EXISTS fk_users_approved_by;
ALTER TABLE public.users DROP CONSTRAINT IF EXISTS fk_users_rejected_by;

ALTER TABLE public.users DROP COLUMN IF EXISTS registration_status;
ALTER TABLE public.users DROP COLUMN IF EXISTS pending_review_message;
ALTER TABLE public.users DROP COLUMN IF EXISTS approved_at;
ALTER TABLE public.users DROP COLUMN IF EXISTS approved_by_user_id;
ALTER TABLE public.users DROP COLUMN IF EXISTS rejected_at;
ALTER TABLE public.users DROP COLUMN IF EXISTS rejected_reason;
ALTER TABLE public.users DROP COLUMN IF EXISTS rejected_by_user_id;
```

### Note on email_outbox

**Decision: Reuse existing `email_outbox` table.** The schema at [`src-tauri/src/models/email_outbox.rs`](src-tauri/src/models/email_outbox.rs) has all needed fields (`tenant_id`, `to_email`, `subject`, `body`, `status`). No new notification table needed.

### Model Changes

**Modify:** [`src-tauri/src/models/user.rs`](src-tauri/src/models/user.rs)

The `User` struct at line 10 uses `#[derive(FromRow)]` via sqlx. Add these fields **after line 49** (after `email_2fa_enabled`):

```rust
// Registration status (active, pending, rejected)
#[serde(default)]
pub registration_status: String,
pub pending_review_message: Option<String>,
pub approved_at: Option<DateTime<Utc>>,
pub approved_by_user_id: Option<String>,
pub rejected_at: Option<DateTime<Utc>>,
pub rejected_reason: Option<String>,
pub rejected_by_user_id: Option<String>,
```

In `User::new()` (line 53), add defaults after line 79:

```rust
registration_status: "active".to_string(), // Changed to "pending" in register() when hybrid flow is active
pending_review_message: None,
approved_at: None,
approved_by_user_id: None,
rejected_at: None,
rejected_reason: None,
rejected_by_user_id: None,
```

In `UserResponse` (line 94), add:

```rust
pub registration_status: String,
```

In the `From<User> for UserResponse` impl (line 114), add to the `Self { ... }` block:

```rust
registration_status: user.registration_status,
```

---

## 4. Backend Service Changes

### 4.1 AppError Variant

**Modify:** [`src-tauri/src/error.rs`](src-tauri/src/error.rs)

Add new variant after line 57 (`Conflict`):

```rust
#[error("Account pending approval")]
AccountPendingApproval,
```

In [`src-tauri/src/http/auth.rs`](src-tauri/src/http/auth.rs) `IntoResponse` impl (line 26), add match arm after the `Conflict` arm (line 60):

```rust
crate::error::AppError::AccountPendingApproval => {
    (StatusCode::FORBIDDEN, "Account pending approval".to_string())
}
```

### 4.2 Modify `register()` in auth_service

**Modify:** [`src-tauri/src/services/auth_service/mod.rs`](src-tauri/src/services/auth_service/mod.rs) — `register_with_email_verification_policy` at line 522.

**Change:** In the INSERT query at line 569, add `registration_status` to the column list:

```sql
INSERT INTO users (id, email, password_hash, name, role, is_active, failed_login_attempts,
    created_at, updated_at, verification_token, email_verified_at, registration_status)
VALUES ($1, $2, $3, $4, $5, $6, 0, $7, $8, $9, $10, $11)
```

**Change:** Before the INSERT (around line 558), set `registration_status` on the user:

```rust
// Hybrid flow: all public registrations are pending
user.registration_status = "pending".to_string();
user.is_active = false; // Pending users are NOT active until approved
```

**Add:** After the INSERT succeeds (after line 596), enqueue notification email to superadmin:

```rust
// Notify superadmin(s) about pending registration
let superadmin_emails: Vec<String> = sqlx::query_scalar(
    "SELECT email FROM users WHERE is_super_admin = true AND is_active = true"
)
.fetch_all(&self.pool)
.await
.unwrap_or_default();

for admin_email in superadmin_emails {
    let subject = format!("New user registration pending approval: {}", user.email);
    let body = format!(
        "A new user has registered and is awaiting your approval.\n\n\
         Name: {}\nEmail: {}\nRegistered at: {}\n\n\
         Please review and approve or reject this registration in the admin panel.",
        user.name, user.email, user.created_at.to_rfc3339()
    );
    if let Err(e) = self.email_service.send_email(&admin_email, &subject, &body).await {
        warn!("Failed to send pending registration notification to {}: {}", admin_email, e);
    }
}
```

**Change:** The return value — both branches (email verification on/off) should return a pending response instead of a token:

```rust
// Always return pending status — no token issued
Ok(AuthResponse {
    user: user.into(),
    tenant: None,
    token: None,
    expires_at: None,
    message: Some("Registration successful. Your account is pending approval by an administrator. You will be able to login once approved.".to_string()),
    requires_2fa: None,
    requires_2fa_setup: None,
    temp_token: None,
    available_2fa_methods: None,
})
```

**Remove:** The existing `generate_token` call at line 648 and the branch that issues a JWT for non-email-verification flow (lines 646-673). Replace with the pending response above.

### 4.3 Modify `login()` in auth_service

**Modify:** [`src-tauri/src/services/auth_service/mod.rs`](src-tauri/src/services/auth_service/mod.rs) — `login` at line 844.

**Add check after `is_active` check (after line 930, before email verification check):**

```rust
// Check registration status — pending users cannot login
if user.registration_status == "pending" {
    let details = serde_json::json!({
        "email": user.email,
        "reason": "account_pending_approval"
    })
    .to_string();
    self.audit_service
        .log(
            Some(&user.id),
            None,
            "login_pending_approval",
            "auth",
            None,
            Some(details.as_str()),
            ip_address.as_deref(),
        )
        .await;
    return Err(AppError::AccountPendingApproval);
}

if user.registration_status == "rejected" {
    let details = serde_json::json!({
        "email": user.email,
        "reason": "account_rejected"
    })
    .to_string();
    self.audit_service
        .log(
            Some(&user.id),
            None,
            "login_rejected",
            "auth",
            None,
            Some(details.as_str()),
            ip_address.as_deref(),
        )
        .await;
    return Err(AppError::Validation("Your registration has been rejected. Please contact support.".to_string()));
}
```

### 4.4 New Method: `approve_pending_user`

**Add to:** [`src-tauri/src/services/auth_service/mod.rs`](src-tauri/src/services/auth_service/mod.rs) — in `impl AuthService` block.

```rust
/// Approve a pending user: set registration_status='active', attach to tenant with role.
pub async fn approve_pending_user(
    &self,
    actor_user_id: &str,      // superadmin performing the action
    target_user_id: &str,     // pending user
    tenant_id: &str,          // tenant to assign
    role_id: &str,            // role to assign within tenant
) -> AppResult<()> {
    let now = Utc::now();

    // Optimistic lock: only approve if still pending
    #[cfg(feature = "postgres")]
    let affected = sqlx::query(
        r#"UPDATE users
           SET registration_status = 'active',
               is_active = true,
               approved_at = $1,
               approved_by_user_id = $2,
               updated_at = $3
           WHERE id = $4 AND registration_status = 'pending'"#
    )
    .bind(now)
    .bind(actor_user_id)
    .bind(now)
    .bind(target_user_id)
    .execute(&self.pool)
    .await?
    .rows_affected();

    #[cfg(not(feature = "postgres"))]
    let affected = {
        let now_str = now.to_rfc3339();
        sqlx::query(
            r#"UPDATE users
               SET registration_status = 'active',
                   is_active = 1,
                   approved_at = ?,
                   approved_by_user_id = ?,
                   updated_at = ?
               WHERE id = ? AND registration_status = 'pending'"#
        )
        .bind(&now_str)
        .bind(actor_user_id)
        .bind(&now_str)
        .bind(target_user_id)
        .execute(&self.pool)
        .await?
        .rows_affected()
    };

    if affected == 0 {
        return Err(AppError::NotFound("User not found or not in pending state".to_string()));
    }

    // Attach user to tenant via tenant_members
    let member_id = Uuid::new_v4().to_string();

    #[cfg(feature = "postgres")]
    sqlx::query(
        r#"INSERT INTO tenant_members (id, tenant_id, user_id, role, role_id, created_at)
           VALUES ($1, $2, $3, 'Member', $4, $5)
           ON CONFLICT DO NOTHING"#
    )
    .bind(&member_id)
    .bind(tenant_id)
    .bind(target_user_id)
    .bind(role_id)
    .bind(now)
    .execute(&self.pool)
    .await?;

    #[cfg(not(feature = "postgres"))]
    {
        let now_str = now.to_rfc3339();
        sqlx::query(
            r#"INSERT OR IGNORE INTO tenant_members (id, tenant_id, user_id, role, role_id, created_at)
               VALUES (?, ?, ?, 'Member', ?, ?)"#
        )
        .bind(&member_id)
        .bind(tenant_id)
        .bind(target_user_id)
        .bind(role_id)
        .bind(&now_str)
        .execute(&self.pool)
        .await?;
    }

    // Audit log
    self.audit_service
        .log(
            Some(actor_user_id),
            Some(tenant_id),
            "user.registration_approved",
            "users",
            Some(target_user_id),
            Some(&format!("Approved pending user and assigned to tenant with role_id={}", role_id)),
            None,
        )
        .await;

    Ok(())
}
```

### 4.5 New Method: `reject_pending_user`

```rust
/// Reject a pending user: mark as rejected (soft, not deleted for audit trail).
pub async fn reject_pending_user(
    &self,
    actor_user_id: &str,
    target_user_id: &str,
    reason: &str,
) -> AppResult<()> {
    let now = Utc::now();

    #[cfg(feature = "postgres")]
    let affected = sqlx::query(
        r#"UPDATE users
           SET registration_status = 'rejected',
               rejected_at = $1,
               rejected_by_user_id = $2,
               rejected_reason = $3,
               updated_at = $4
           WHERE id = $5 AND registration_status = 'pending'"#
    )
    .bind(now)
    .bind(actor_user_id)
    .bind(reason)
    .bind(now)
    .bind(target_user_id)
    .execute(&self.pool)
    .await?
    .rows_affected();

    #[cfg(not(feature = "postgres"))]
    let affected = {
        let now_str = now.to_rfc3339();
        sqlx::query(
            r#"UPDATE users
               SET registration_status = 'rejected',
                   rejected_at = ?,
                   rejected_by_user_id = ?,
                   rejected_reason = ?,
                   updated_at = ?
               WHERE id = ? AND registration_status = 'pending'"#
        )
        .bind(&now_str)
        .bind(actor_user_id)
        .bind(reason)
        .bind(&now_str)
        .bind(target_user_id)
        .execute(&self.pool)
        .await?
        .rows_affected()
    };

    if affected == 0 {
        return Err(AppError::NotFound("User not found or not in pending state".to_string()));
    }

    self.audit_service
        .log(
            Some(actor_user_id),
            None,
            "user.registration_rejected",
            "users",
            Some(target_user_id),
            Some(&format!("Rejected with reason: {}", reason)),
            None,
        )
        .await;

    Ok(())
}
```

### 4.6 New Method: `list_pending_users`

```rust
/// List all users with registration_status='pending'.
pub async fn list_pending_users(&self) -> AppResult<Vec<User>> {
    let users = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE registration_status = 'pending' ORDER BY created_at DESC"
    )
    .fetch_all(&self.pool)
    .await?;

    Ok(users)
}
```

---

## 5. HTTP Endpoint Changes

### 5.1 Register Endpoint — Response Change

**Modify:** [`src-tauri/src/http/auth.rs`](src-tauri/src/http/auth.rs) — `register` handler at line 132.

**No change needed** — the handler already returns `Json<AuthResponse>`. Since `register()` now returns `AuthResponse` with `token: None` and a pending message, the response shape is automatically:

```json
{
  "user": { "id": "...", "email": "...", "registration_status": "pending", ... },
  "tenant": null,
  "token": null,
  "expires_at": null,
  "message": "Registration successful. Your account is pending approval by an administrator."
}
```

The frontend can detect `registration_status === 'pending'` from the `user` object and `token === null`.

### 5.2 New File: Superadmin Registration Approvals

**Create:** [`src-tauri/src/http/registration_approvals.rs`](src-tauri/src/http/registration_approvals.rs)

Pattern follows [`src-tauri/src/http/superadmin.rs`](src-tauri/src/http/superadmin.rs) — use `extract_super_admin` helper (line 262 of superadmin.rs) for auth gating.

```rust
use super::AppState;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;

/// Reuse the superadmin auth pattern
async fn require_super_admin(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<crate::services::auth_service::Claims, crate::error::AppError> {
    let token = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(crate::error::AppError::Unauthorized)?;

    let claims = state.auth_service.validate_token(token).await?;

    if !claims.is_super_admin {
        return Err(crate::error::AppError::Unauthorized);
    }

    Ok(claims)
}

/// GET /api/superadmin/registration-approvals
pub async fn list_pending(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let _claims = require_super_admin(&state, &headers).await?;

    let pending_users = state.auth_service.list_pending_users().await?;

    let items: Vec<serde_json::Value> = pending_users
        .into_iter()
        .map(|u| {
            serde_json::json!({
                "id": u.id,
                "email": u.email,
                "name": u.name,
                "pending_review_message": u.pending_review_message,
                "created_at": u.created_at.to_rfc3339(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "users": items,
        "total": items.len(),
    })))
}

#[derive(Deserialize)]
pub struct ApproveDto {
    pub tenant_id: String,
    pub role_id: String,
}

/// POST /api/superadmin/registration-approvals/{user_id}/approve
pub async fn approve(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(payload): Json<ApproveDto>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let claims = require_super_admin(&state, &headers).await?;

    state
        .auth_service
        .approve_pending_user(&claims.sub, &user_id, &payload.tenant_id, &payload.role_id)
        .await?;

    Ok(Json(serde_json::json!({
        "message": "User approved successfully"
    })))
}

#[derive(Deserialize)]
pub struct RejectDto {
    pub reason: String,
}

/// POST /api/superadmin/registration-approvals/{user_id}/reject
pub async fn reject(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(payload): Json<RejectDto>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let claims = require_super_admin(&state, &headers).await?;

    state
        .auth_service
        .reject_pending_user(&claims.sub, &user_id, &payload.reason)
        .await?;

    Ok(Json(serde_json::json!({
        "message": "User rejected"
    })))
}
```

### 5.3 Register Module & Routes

**Modify:** [`src-tauri/src/http/mod.rs`](src-tauri/src/http/mod.rs) — add module declaration:

```rust
pub mod registration_approvals;
```

**Modify:** [`src-tauri/src/bootstrap/http.rs`](src-tauri/src/bootstrap/http.rs) — add routes after the existing `/api/superadmin/` routes (around line 493):

```rust
// Registration Approvals
.route(
    "/api/superadmin/registration-approvals",
    get(registration_approvals::list_pending),
)
.route(
    "/api/superadmin/registration-approvals/{user_id}/approve",
    post(registration_approvals::approve),
)
.route(
    "/api/superadmin/registration-approvals/{user_id}/reject",
    post(registration_approvals::reject),
)
```

Ensure `use crate::http::registration_approvals;` is added to the imports.

---

## 6. Email Notification

### Approach

Use **direct email send** via `self.email_service.send_email()` inside `register()` (see section 4.2). This is consistent with the existing pattern at lines 603-617 of [`auth_service/mod.rs`](src-tauri/src/services/auth_service/mod.rs) for verification emails. If `email_outbox` is preferred for reliability, the executor can swap to `email_outbox_service.send_or_enqueue()` — but the direct approach matches existing codebase patterns.

### Notification Recipients

All users where `is_super_admin = true AND is_active = true`. Query at registration time.

### No Template Seed Needed

The notification email is a simple plain-text email (name, email, date, link to admin panel). A message template can be added in a future iteration. For MVP, inline format string is sufficient and matches existing patterns (verification email, reset password email).

---

## 7. Frontend Changes

### 7.1 Register Page — Handle Pending Response

**Modify:** [`src/routes/register/+page.svelte`](src/routes/register/+page.svelte)

The submit handler (search for the `register` or form submission logic, approximately line 200+) currently expects a successful response with a token. After registration response:

```typescript
// After successful register call
if (response.user?.registration_status === 'pending') {
  // Show pending approval message, do NOT store token or redirect to dashboard
  pendingApproval = true;
  // Optionally disable form
  return;
}
```

Add state variable at the top of `<script>`:

```typescript
let pendingApproval = false;
```

Add conditional UI block in the template (replace or wrap the success flow):

```svelte
{#if pendingApproval}
  <div class="text-center py-8">
    <Icon name="clock" class="w-12 h-12 text-yellow-500 mx-auto mb-4" />
    <h2 class="text-xl font-semibold mb-2">{$t('auth.register.pending_title')}</h2>
    <p class="text-gray-600 dark:text-gray-400 mb-6">{$t('auth.register.pending_message')}</p>
    <a href="/login" class="btn btn-primary">{$t('auth.register.back_to_login')}</a>
  </div>
{:else}
  <!-- existing form -->
{/if}
```

### 7.2 Login Page — Handle Pending Error

**Modify:** [`src/routes/login/+page.svelte`](src/routes/login/+page.svelte)

The login handler catches errors. In the catch block (search for `error =` assignment after login call), add:

```typescript
catch (e: any) {
  const msg = e?.message || e?.toString() || '';
  if (msg.includes('Account pending approval') || msg.includes('AccountPendingApproval')) {
    error = $t('auth.login.error_pending_approval');
  } else if (msg.includes('rejected')) {
    error = $t('auth.login.error_rejected');
  } else {
    error = msg || $t('auth.login.error_generic');
  }
}
```

### 7.3 New Superadmin Page: Pending Approvals

**Create:** `src/routes/superadmin/registration-approvals/+page.svelte`

Pattern follows existing superadmin pages like [`src/routes/superadmin/users/+page.svelte`](src/routes/superadmin/users/+page.svelte). Structure:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from 'svelte-i18n';
  import { superadminApi } from '$lib/api/superadmin';
  // ... component logic: fetch list, approve/reject actions
</script>

<div class="p-6">
  <h1 class="text-2xl font-bold mb-6">{$t('superadmin.pending_approvals.title')}</h1>
  <!-- Table of pending users with approve/reject buttons -->
  <!-- Approve dialog: select tenant + role -->
  <!-- Reject dialog: enter reason -->
</div>
```

### 7.4 API Client Wrapper

**Modify:** [`src/lib/api/superadmin.ts`](src/lib/api/superadmin.ts)

Add methods:

```typescript
listPendingApprovals: (): Promise<{ users: PendingUser[]; total: number }> =>
  safeInvoke('superadmin_list_pending_approvals', { token: getTokenOrThrow() }),

approvePendingUser: (userId: string, tenantId: string, roleId: string): Promise<void> =>
  safeInvoke('superadmin_approve_pending_user', { token: getTokenOrThrow(), userId, tenantId, roleId }),

rejectPendingUser: (userId: string, reason: string): Promise<void> =>
  safeInvoke('superadmin_reject_pending_user', { token: getTokenOrThrow(), userId, reason }),
```

**Note:** If this project uses HTTP endpoints (Axum) rather than Tauri `invoke` commands for API calls, the client should use `fetch` or `axios` against `/api/superadmin/registration-approvals` endpoints instead. Verify the pattern in existing `superadmin.ts` — the current codebase uses Tauri `safeInvoke` for most operations but HTTP routes for superadmin. Check [`src/lib/api/superadmin.ts`](src/lib/api/superadmin.ts) for the actual pattern used.

**Alternative (if using direct HTTP):**

```typescript
listPendingApprovals: async (): Promise<{ users: PendingUser[]; total: number }> => {
  const res = await fetch('/api/superadmin/registration-approvals', {
    headers: { 'Authorization': `Bearer ${getTokenOrThrow()}` },
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
},
```

### 7.5 i18n Keys

**Modify:** [`src/lib/i18n/namespaces/en/auth.json`](src/lib/i18n/namespaces/en/auth.json)

Add inside `"register"` object (after `"login_link"` at line 31):

```json
"pending_title": "Registration Pending",
"pending_message": "Your account has been created and is awaiting approval by an administrator. You will receive an email once your account is approved.",
"back_to_login": "Back to Login"
```

Add inside `"login"` object (after `"register_link"` at line 14):

```json
"error_pending_approval": "Your account is pending approval. Please wait for an administrator to review your registration.",
"error_rejected": "Your registration has been rejected. Please contact support for assistance."
```

**Modify:** [`src/lib/i18n/namespaces/id/auth.json`](src/lib/i18n/namespaces/id/auth.json)

Add inside `"register"`:

```json
"pending_title": "Pendaftaran Tertunda",
"pending_message": "Akun Anda telah dibuat dan sedang menunggu persetujuan dari administrator. Anda akan menerima email setelah akun disetujui.",
"back_to_login": "Kembali ke Login"
```

Add inside `"login"`:

```json
"error_pending_approval": "Akun Anda sedang menunggu persetujuan. Silakan tunggu administrator meninjau pendaftaran Anda.",
"error_rejected": "Pendaftaran Anda telah ditolak. Silakan hubungi dukungan untuk bantuan."
```

**Modify:** [`src/lib/i18n/namespaces/en/superadmin.json`](src/lib/i18n/namespaces/en/superadmin.json)

Add:

```json
"pending_approvals": {
  "title": "Pending Registrations",
  "empty": "No pending registrations",
  "approve": "Approve",
  "reject": "Reject",
  "approve_dialog_title": "Approve User",
  "reject_dialog_title": "Reject User",
  "select_tenant": "Assign to Tenant",
  "select_role": "Assign Role",
  "reject_reason": "Reason for Rejection",
  "confirm_approve": "Confirm Approval",
  "confirm_reject": "Confirm Rejection",
  "approved_success": "User approved successfully",
  "rejected_success": "User rejected successfully"
}
```

**Modify:** [`src/lib/i18n/namespaces/id/superadmin.json`](src/lib/i18n/namespaces/id/superadmin.json)

Add equivalent Indonesian translations.

---

## 8. Tests (TDD Per Layer)

### 8.1 Backend Integration Tests

**Modify:** [`src-tauri/src/services/auth_service/tests.rs`](src-tauri/src/services/auth_service/tests.rs)

Each test uses the existing test helpers in that file.

#### Test 1: `register_with_status_creates_pending_user`

```rust
#[tokio::test]
async fn register_with_status_creates_pending_user() {
    let (pool, auth_service) = setup_test_env().await;

    let dto = RegisterDto {
        email: "newuser@test.com".to_string(),
        password: "Str0ngP@ss!".to_string(),
        name: "Test User".to_string(),
    };

    let response = auth_service.register(dto, None).await.unwrap();

    assert!(response.token.is_none());
    assert!(response.message.unwrap().contains("pending"));
    assert_eq!(response.user.registration_status, "pending");

    // Verify in DB
    let user: User = sqlx::query_as("SELECT * FROM users WHERE email = $1")
        .bind("newuser@test.com")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(user.registration_status, "pending");
    assert!(!user.is_active);
}
```

#### Test 2: `pending_user_cannot_login`

```rust
#[tokio::test]
async fn pending_user_cannot_login() {
    let (pool, auth_service) = setup_test_env().await;

    // Register a user (creates pending)
    let dto = RegisterDto {
        email: "pending@test.com".to_string(),
        password: "Str0ngP@ss!".to_string(),
        name: "Pending User".to_string(),
    };
    auth_service.register(dto, None).await.unwrap();

    // Attempt login
    let login_dto = LoginDto {
        email: "pending@test.com".to_string(),
        password: "Str0ngP@ss!".to_string(),
    };
    let result = auth_service.login(login_dto, None, None).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(format!("{}", err).contains("pending approval"));
}
```

#### Test 3: `approve_pending_user_attaches_to_tenant_with_role`

```rust
#[tokio::test]
async fn approve_pending_user_attaches_to_tenant_with_role() {
    let (pool, auth_service) = setup_test_env().await;

    // Create superadmin actor
    let superadmin_id = create_test_superadmin(&pool).await;
    // Create target tenant
    let tenant_id = create_test_tenant(&pool).await;
    // Get Owner role_id
    let role_id: String = sqlx::query_scalar("SELECT id FROM roles WHERE name = 'Member' AND tenant_id IS NULL LIMIT 1")
        .fetch_one(&pool).await.unwrap();

    // Register pending user
    let dto = RegisterDto { email: "approve@test.com".to_string(), password: "Str0ngP@ss!".to_string(), name: "Approve Me".to_string() };
    let reg = auth_service.register(dto, None).await.unwrap();
    let user_id = reg.user.id;

    // Approve
    auth_service.approve_pending_user(&superadmin_id, &user_id, &tenant_id, &role_id).await.unwrap();

    // Verify
    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(&user_id).fetch_one(&pool).await.unwrap();
    assert_eq!(user.registration_status, "active");
    assert!(user.is_active);
    assert!(user.approved_at.is_some());
    assert_eq!(user.approved_by_user_id, Some(superadmin_id));

    // Verify tenant membership
    let member_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tenant_members WHERE user_id = $1 AND tenant_id = $2)")
        .bind(&user_id).bind(&tenant_id).fetch_one(&pool).await.unwrap();
    assert!(member_exists);
}
```

#### Test 4: `approve_pending_user_emits_audit_log`

```rust
#[tokio::test]
async fn approve_pending_user_emits_audit_log() {
    let (pool, auth_service) = setup_test_env().await;
    let superadmin_id = create_test_superadmin(&pool).await;
    let tenant_id = create_test_tenant(&pool).await;
    let role_id = get_member_role_id(&pool).await;

    let dto = RegisterDto { email: "audit@test.com".to_string(), password: "Str0ngP@ss!".to_string(), name: "Audit Test".to_string() };
    let reg = auth_service.register(dto, None).await.unwrap();

    auth_service.approve_pending_user(&superadmin_id, &reg.user.id, &tenant_id, &role_id).await.unwrap();

    let audit: String = sqlx::query_scalar("SELECT action FROM audit_logs WHERE resource_id = $1 ORDER BY created_at DESC LIMIT 1")
        .bind(&reg.user.id).fetch_one(&pool).await.unwrap();
    assert_eq!(audit, "user.registration_approved");
}
```

#### Test 5: `reject_pending_user_marks_rejected_and_emits_audit_log`

```rust
#[tokio::test]
async fn reject_pending_user_marks_rejected_and_emits_audit_log() {
    let (pool, auth_service) = setup_test_env().await;
    let superadmin_id = create_test_superadmin(&pool).await;

    let dto = RegisterDto { email: "reject@test.com".to_string(), password: "Str0ngP@ss!".to_string(), name: "Reject Test".to_string() };
    let reg = auth_service.register(dto, None).await.unwrap();

    auth_service.reject_pending_user(&superadmin_id, &reg.user.id, "spam account").await.unwrap();

    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(&reg.user.id).fetch_one(&pool).await.unwrap();
    assert_eq!(user.registration_status, "rejected");
    assert_eq!(user.rejected_reason, Some("spam account".to_string()));

    let audit: String = sqlx::query_scalar("SELECT action FROM audit_logs WHERE resource_id = $1 ORDER BY created_at DESC LIMIT 1")
        .bind(&reg.user.id).fetch_one(&pool).await.unwrap();
    assert_eq!(audit, "user.registration_rejected");
}
```

#### Test 6: `superadmin_can_list_pending_users`

```rust
#[tokio::test]
async fn superadmin_can_list_pending_users() {
    let (pool, auth_service) = setup_test_env().await;

    // Register 2 pending users
    let dto1 = RegisterDto { email: "list1@test.com".to_string(), password: "Str0ngP@ss!".to_string(), name: "List1".to_string() };
    let dto2 = RegisterDto { email: "list2@test.com".to_string(), password: "Str0ngP@ss!".to_string(), name: "List2".to_string() };
    auth_service.register(dto1, None).await.unwrap();
    auth_service.register(dto2, None).await.unwrap();

    let pending = auth_service.list_pending_users().await.unwrap();
    assert_eq!(pending.len(), 2);
    assert!(pending.iter().all(|u| u.registration_status == "pending"));
}
```

#### Test 7: `non_superadmin_cannot_approve`

Test at the HTTP layer — see section 8.2.

### 8.2 HTTP Layer Tests (optional integration)

Test in [`src-tauri/src/http/registration_approvals.rs`](src-tauri/src/http/registration_approvals.rs) with `#[cfg(test)]` module or in a dedicated test file. Key scenario:

- Call `POST /api/superadmin/registration-approvals/{id}/approve` without superadmin token → expect `401 Unauthorized`.

### 8.3 Frontend Tests

**Create:** `src/routes/register-pending-ui.test.ts` (or add to existing [`src/routes/public-auth-ui.test.ts`](src/routes/public-auth-ui.test.ts))

```typescript
// Test: register page shows pending message when response has registration_status='pending'
// Test: login page shows "Account pending approval" when error contains AccountPendingApproval
// Test: registration approvals page renders list and approve/reject buttons (gated by superadmin)
```

---

## 9. Migration & Rollout Steps

### Recommended Commit Order

Each chunk is independently testable and deployable.

#### Chunk 1: Schema Migration (DB only, no code impact)
- Create migration files (`20260529130000_add_user_registration_status.{up,down}.sql`)
- Run migration
- Verify: `SELECT column_name FROM information_schema.columns WHERE table_name = 'users' AND column_name = 'registration_status';`

#### Chunk 2: Model Layer
- Modify [`src-tauri/src/models/user.rs`](src-tauri/src/models/user.rs) — add new fields to `User`, `UserResponse`, `User::new()`
- `cargo check` — compile passes (new fields have defaults, existing code unaffected)

#### Chunk 3: Backend Service Core (TDD)
- Add `AccountPendingApproval` variant to [`src-tauri/src/error.rs`](src-tauri/src/error.rs)
- Modify `register()` in [`src-tauri/src/services/auth_service/mod.rs`](src-tauri/src/services/auth_service/mod.rs)
- Modify `login()` — add registration_status check
- Add `approve_pending_user()`, `reject_pending_user()`, `list_pending_users()`
- Write and run tests in `auth_service/tests.rs`
- Commit with passing tests

#### Chunk 4: HTTP Handlers
- Create [`src-tauri/src/http/registration_approvals.rs`](src-tauri/src/http/registration_approvals.rs)
- Add `AccountPendingApproval` match arm in [`src-tauri/src/http/auth.rs`](src-tauri/src/http/auth.rs) `IntoResponse`
- Register routes in [`src-tauri/src/bootstrap/http.rs`](src-tauri/src/bootstrap/http.rs)
- Add module in [`src-tauri/src/http/mod.rs`](src-tauri/src/http/mod.rs)
- `cargo check` + `cargo test`
- Commit

#### Chunk 5: Frontend — Register & Login Flow
- Modify [`src/routes/register/+page.svelte`](src/routes/register/+page.svelte) — pending state UI
- Modify [`src/routes/login/+page.svelte`](src/routes/login/+page.svelte) — error handling
- Add i18n keys to `en/auth.json` and `id/auth.json`
- `npm run check`
- Commit

#### Chunk 6: Frontend — Superadmin Pending Approvals Page
- Create `src/routes/superadmin/registration-approvals/+page.svelte`
- Add API methods to [`src/lib/api/superadmin.ts`](src/lib/api/superadmin.ts)
- Add i18n keys to `en/superadmin.json` and `id/superadmin.json`
- `npm run check`
- Commit

#### Chunk 7: Documentation
- Update [`FEATURES.md`](FEATURES.md) — add Hybrid Registration Flow entry
- Update [`SYSTEM_MAP.md`](SYSTEM_MAP.md) if exists — document new endpoints and flow
- Commit

---

## 10. Acceptance Criteria

- [ ] Public register creates user with `registration_status='pending'`, `is_active=false`, no JWT returned
- [ ] Login attempt by pending user returns `403` with `"Account pending approval"` message
- [ ] Login attempt by rejected user returns `400` with `"Your registration has been rejected"` message
- [ ] `GET /api/superadmin/registration-approvals` returns list of pending users (superadmin only)
- [ ] `POST /api/superadmin/registration-approvals/{id}/approve` sets user to active, attaches tenant_member row, emits audit log
- [ ] `POST /api/superadmin/registration-approvals/{id}/reject` sets user to rejected with reason, emits audit log
- [ ] Non-superadmin cannot access registration approval endpoints (returns 401)
- [ ] Email notification sent to superadmin on new pending registration
- [ ] Frontend register page shows "Registration Pending" message (no token stored)
- [ ] Frontend login page shows "Account pending approval" error for pending users
- [ ] Frontend superadmin page shows pending list with approve/reject actions
- [ ] All existing `cargo test` pass (no regression)
- [ ] All existing `vitest` pass (no regression)
- [ ] `npm run check` passes (Svelte diagnostics clean)
- [ ] Migration up/down runs cleanly

---

## 11. Open Questions / Decisions

### Decision 1: Global Pending vs Per-Tenant Pending

**Decision made: Global pending (superadmin-only approval).** Public `/register` has no tenant context. Pending users are orphaned by design until superadmin assigns them. Tenant-scoped invite-token flow (existing in `customer_service/registration.rs`) handles per-tenant flows separately.

**Extension point:** The `approve_pending_user` method takes `tenant_id` as parameter, so a future enhancement could let tenant Owners also approve if we add a `users:approve_registrations` permission check.

### Decision 2: Email Notification Approach

**Decision made: Direct send via `email_service.send_email()` inside `register()`.** This matches existing patterns for verification and reset password emails. The `email_outbox` can be adopted later for retry reliability if needed — the method signature in `EmailOutboxService::send_or_enqueue()` is drop-in compatible.

### Decision 3: Rejected User Retention

**Decision made: Soft reject (mark as rejected, no deletion).** Rejected users remain in DB for audit trail. No automatic cleanup. Future enhancement could add a cron job to purge rejected users older than N days.

### Decision 4: Re-registration with Rejected Email

**Decision made: Allowed.** If a user with a rejected email tries to re-register, the existing "User already exists" check at line 551 will block them. **This is acceptable behavior** — rejected users should contact support rather than re-register. If re-registration is desired, a future enhancement could allow re-registration by resetting the rejected user to pending.

### Open Question 1: Should tenant Owners (not just superadmin) be able to approve pending users?

**Default answer: No, not in MVP.** This keeps scope tight. The endpoint is under `/api/superadmin/` and gated by `claims.is_super_admin`. If tenant Owner approval is needed, it can be added as a separate feature with the `users:approve_registrations` permission.

### Open Question 2: Should we add `auth_allow_registration` setting check that differentiates "pending flow" vs "disabled"?

**Default answer: No.** The existing `allow_registration` setting gates whether registration is possible at all. When `true`, hybrid pending flow is always active. No new setting needed.

---

## 12. Risks & Mitigations

### Risk 1: Existing tenant Owners with `allow_registration=true` expecting auto-active users

**Impact:** Tenant Owners who previously relied on public registration giving instant access will now get pending users they cannot approve (only superadmin can).  
**Mitigation:** Document in release notes. Add a UI warning on the `allow_registration` settings toggle: "Registered users will require superadmin approval before they can login."

### Risk 2: Race condition — two superadmin sessions approve same user

**Impact:** Double INSERT into `tenant_members` (duplicate membership).  
**Mitigation:** The SQL `WHERE registration_status = 'pending'` in the UPDATE acts as optimistic lock. If the first approval sets it to `'active'`, the second will find `rows_affected() == 0` and return `NotFound`. The `ON CONFLICT DO NOTHING` on `tenant_members` INSERT provides a second safety net.

### Risk 3: Superadmin email notification spam on high registration volume

**Impact:** If many bots register, superadmin inbox floods.  
**Mitigation:** Rate limiting already exists at 10 requests/60s for `/api/auth/register` (see [`src-tauri/src/http/middleware.rs:173`](src-tauri/src/http/middleware.rs:173)). Email is only sent for valid registrations (password policy check, duplicate check). For future: add email dedup or digest mode.

### Risk 4: SQLx compile-time query checking fails with new columns

**Impact:** If `SELECT *` queries are compile-time checked by SQLx, the new columns must be present in the schema when building.  
**Mitigation:** Run migration before `cargo build`. All new columns have `DEFAULT` values, so existing data is unaffected. The `User` struct uses `sqlx::FromRow` which maps columns by name, not position.

### Risk 5: Frontend stores token even when `null`

**Impact:** If the register handler doesn't check for `token === null`, it might store `null` as the auth token.  
**Mitigation:** The register page change (section 7.1) explicitly checks `registration_status === 'pending'` before any token handling. The existing auth store likely guards against null tokens, but verify during implementation.

---

## Appendix: Files Summary

### Files to Create
| File | Purpose |
|------|---------|
| `src-tauri/migrations/20260529130000_add_user_registration_status.up.sql` | Schema migration (ALTER users) |
| `src-tauri/migrations/20260529130000_add_user_registration_status.down.sql` | Rollback migration |
| `src-tauri/src/http/registration_approvals.rs` | Superadmin HTTP endpoints |
| `src/routes/superadmin/registration-approvals/+page.svelte` | Admin approval UI page |

### Files to Modify
| File | Changes |
|------|---------|
| [`src-tauri/src/models/user.rs`](src-tauri/src/models/user.rs) | Add registration_status + audit fields to User/UserResponse |
| [`src-tauri/src/error.rs`](src-tauri/src/error.rs) | Add `AccountPendingApproval` variant |
| [`src-tauri/src/services/auth_service/mod.rs`](src-tauri/src/services/auth_service/mod.rs) | Modify register/login, add approve/reject/list methods |
| [`src-tauri/src/http/auth.rs`](src-tauri/src/http/auth.rs) | Add `AccountPendingApproval` to IntoResponse |
| [`src-tauri/src/http/mod.rs`](src-tauri/src/http/mod.rs) | Add `registration_approvals` module |
| [`src-tauri/src/bootstrap/http.rs`](src-tauri/src/bootstrap/http.rs) | Register new routes |
| [`src-tauri/src/services/auth_service/tests.rs`](src-tauri/src/services/auth_service/tests.rs) | Add 6+ integration tests |
| [`src/routes/register/+page.svelte`](src/routes/register/+page.svelte) | Pending state UI |
| [`src/routes/login/+page.svelte`](src/routes/login/+page.svelte) | Pending/rejected error handling |
| [`src/lib/api/superadmin.ts`](src/lib/api/superadmin.ts) | Add API methods for approval endpoints |
| [`src/lib/i18n/namespaces/en/auth.json`](src/lib/i18n/namespaces/en/auth.json) | Add pending/rejected i18n keys |
| [`src/lib/i18n/namespaces/id/auth.json`](src/lib/i18n/namespaces/id/auth.json) | Add pending/rejected i18n keys |
| [`src/lib/i18n/namespaces/en/superadmin.json`](src/lib/i18n/namespaces/en/superadmin.json) | Add approval page i18n keys |
| [`src/lib/i18n/namespaces/id/superadmin.json`](src/lib/i18n/namespaces/id/superadmin.json) | Add approval page i18n keys |
| [`FEATURES.md`](FEATURES.md) | Document hybrid registration flow |
