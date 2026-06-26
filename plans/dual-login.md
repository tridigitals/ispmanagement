# Dual Login: Email + Phone — Implementation Plan

## Goal
User bisa login dengan **email ATAU nomor HP** + password.

## Scope
1. **Rust Backend** — modify `LoginDto` + login query
2. **Svelte Web** — login page dengan toggle email/phone
3. **Flutter Mobile** — customer/admin/technician login screens

## Changes

### 1. Rust Backend

**File:** `src-tauri/src/models/user.rs`
- Ubah `LoginDto`:
  ```rust
  pub struct LoginDto {
      pub identifier: String,  // email OR phone
      pub password: String,
  }
  ```
- Add validation: identifier harus email format ATAU phone format (minimal 8 digit, hanya angka/+)

**File:** `src-tauri/src/services/auth_service/mod.rs`
- Ubah login query:
  ```rust
  // Cari user by email ATAU phone (normalized)
  let user: Option<User> = sqlx::query_as(
    "SELECT * FROM users WHERE email = $1 OR phone = $1"
  )
  .bind(&normalized_identifier)
  .fetch_optional(&self.pool)
  .await?;
  ```

**File:** `src-tauri/src/commands/auth.rs`
- Ubah command signature: `email: String` → `identifier: String`
- Normalize identifier (trim, lowercase untuk email, normalize phone dengan prefix)

### 2. Svelte Web

**File:** `src/routes/login/+page.svelte`
- Tambah state: `loginBy: 'email' | 'phone'`
- Toggle switch: "Login dengan Email" / "Login dengan No HP"
- Input field berubah sesuai toggle:
  - Email: `type="email"`, placeholder "email@example.com"
  - Phone: `type="tel"`, placeholder "08xxxxxxxxxx"
- Kirim `identifier` ke API (bukan `email`)

**File:** `src/lib/api/auth.ts`
- Ubah `login()` signature: `(email: string, ...)` → `(identifier: string, ...)`

### 3. Flutter Mobile (Customer)

**File:** `apps/mobile-customer/lib/src/features/auth/login_screen.dart`
- Tambah dropdown/segmented control: "Email" | "No HP"
- Input berubah sesuai pilihan
- Kirim identifier ke auth controller

**Files to check/update:**
- `apps/mobile-admin/` login
- `apps/mobile-technician/` login

## Phone Normalization
```
Input: "0812 3456 7890" / "+6281234567890" / "081234567890"
Output (stored): "6281234567890" (E.164 format, tanpa spasi)
```

Login identifier akan di-normalize sebelum query:
1. Hapus spasi, dash, parentheses
2. Jika mulai dengan "0", replace jadi "62"
3. Jika mulai dengan "+", remove "+"

## Backward Compatibility
- Login dengan email tetap berfungsi seperti biasa
- Jika ada duplicate phone di `customers` tapi tidak ada di `users`, data di `customers` tetap ada (tidak dihapus) — hanya login yang menggunakan `users.phone`

## Database Check (for duplicate phone cleanup)
```sql
-- Find customers with phone that doesn't exist in users.phone
SELECT c.id, c.phone, c.name 
FROM customers c
WHERE c.phone IS NOT NULL
  AND NOT EXISTS (
    SELECT 1 FROM users u WHERE u.phone = c.phone
  );
```
