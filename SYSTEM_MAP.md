<!--
Tujuan: Peta navigasi utama arsitektur proyek ISPMANAGEMENT untuk trace-by-function dan trace-by-flow.
Caller: Engineer/agent yang memulai sesi analisis, debugging, implementasi, atau review di repo ini.
Dependensi: Struktur repo, entrypoint SvelteKit/Tauri/Axum, service Rust, migration SQL, script/deploy docs.
Main Functions: Menentukan entrypoint, layer utama, flow domain, dan file target tercepat untuk penelusuran.
Side Effects: Tidak ada runtime side effect; dokumen referensi hasil analisis kode.
-->

# SYSTEM MAP

## Mandatory Map Check

Pada awal sesi baru:

1. Baca `SYSTEM_MAP.md` ini dulu.
2. Tentukan app/domain yang relevan.
3. Mulai trace dari entrypoint pada bagian itu.
4. Hindari blind scan file besar; baca blok fungsi yang disebut di peta ini.

Jika peta ini terasa usang terhadap kode aktual, perbarui bagian terkait dulu sebelum analisis lanjut.

## Scope Analisis

Peta ini dibuat dengan targeted trace, bukan full scan file besar.

Exclusion yang diterapkan saat analisis:

- Dependencies/build/cache/artifact: `node_modules`, `dist`, `build`, `.git`, `.vscode`, `.idea`, `coverage`, `.next`, `.nuxt`, `.cache`, `tmp`, `__pycache__`, `vendor`, `target`, `.gradle`, `bin`, `pkg`, `*.log`, `*.lock`, `*.min.*`, `*.map`
- Folder kerja terpisah seperti `.worktrees/*` tidak dipetakan sebagai runtime utama repo ini

## Repo Snapshot

- Primary frontend: SvelteKit SPA di `src/`
- Primary backend/runtime: Rust crate Tauri + Axum di `src-tauri/`
- Runtime modes:
  - Desktop app: Tauri desktop + embedded HTTP API
  - Standalone server: binary Axum `src-tauri/src/bin/server.rs`
  - Utility CLIs: migration/seed/setup binaries di `src-tauri/src/bin/`
- DB mode:
  - Default: PostgreSQL
  - Optional: SQLite via feature flag

## App Groups

### 1. Web/Desktop UI

- Entrypoint shell: `src/routes/+layout.svelte`
- Route rewrite: `src/hooks.ts`
- Root routing mode: SPA via `src/routes/+layout.ts`
- API bridge: `src/lib/api/core.ts`
- Auth/session state: `src/lib/stores/auth.ts`
- Realtime state: `src/lib/stores/websocket.ts`

### 2. HTTP Backend

- HTTP boot: `src-tauri/src/bootstrap/http.rs`
- Shared state: `src-tauri/src/http/mod.rs`
- Desktop backend init: `src-tauri/src/bootstrap/app.rs`
- Standalone server boot: `src-tauri/src/bin/server.rs`

### 3. Data / DB / Migration

- DB abstraction: `src-tauri/src/db/connection/mod.rs`
- DB init/bootstrap: `src-tauri/src/db/connection/bootstrap.rs`
- Migrations: `src-tauri/migrations/`
- Migration runner: `src-tauri/src/bin/migrate.rs`
- Seed runner: `src-tauri/src/bin/seed.rs`

### 4. Ops / Packaging

- Local Postgres: `docker-compose.yml`
- Systemd deployment: `deploy/systemd/`
- FreeRADIUS sidecar/control-plane assets: `deploy/freeradius/`
- Repo utility scripts: `scripts/`

## Fast Start By Intent

### Jika task tentang login / tenant routing / session

Urutan baca:

1. `src/hooks.ts`
2. `src/routes/+layout.svelte`
3. `src/lib/stores/auth.ts`
4. `src/lib/api/core.ts`
5. `src-tauri/src/http/auth.rs`
6. `src-tauri/src/services/auth_service/mod.rs`
7. `src-tauri/src/services/auth_service/core.rs`
8. `src-tauri/src/services/auth_service/repository.rs`

### Jika task tentang customer / portal / work order

Urutan baca:

1. `src/routes/[tenant]/(app)/...customers...`
2. `src/lib/api/customers.ts`
3. `src-tauri/src/http/customers.rs`
4. `src-tauri/src/services/customer_service/core.rs`
5. `src-tauri/src/services/customer_service/repository.rs`

### Jika task tentang network map / coverage / topology

Urutan baca:

1. `src/routes/[tenant]/(app)/admin/network/map/+page.svelte`
2. `src/lib/components/network/*`
3. `src/lib/api/networkMapping.ts`
4. `src-tauri/src/http/network_mapping.rs`
5. `src-tauri/src/services/network_mapping_service/mod.rs`
6. `src-tauri/src/services/network_mapping_service/core.rs`
7. `src-tauri/src/services/network_mapping_service/repository.rs`

### Jika task tentang MixRadius import

Urutan baca:

1. `src/routes/[tenant]/(app)/admin/network/import/mixradius/+page.svelte`
2. `src/lib/components/network/mixradius/MixRadiusImportWizard.svelte`
3. `src/lib/api/mixradiusImport.ts`
4. `src-tauri/src/http/mixradius_import.rs`
5. `src-tauri/src/services/mixradius_import_service/mod.rs`
6. `src-tauri/src/services/mixradius_import_executor.rs`
7. `src-tauri/src/services/mixradius_sql_parser.rs`

### Jika task tentang invoice / payment / billing collection

Urutan baca:

1. `src/routes/[tenant]/(app)/admin/invoices/+page.svelte` atau portal invoice pages
2. `src/lib/api/payment.ts`
3. `src-tauri/src/http/payment.rs`
4. `src-tauri/src/services/payment_service/mod.rs`
5. `src-tauri/src/services/payment_service/core.rs`
6. `src-tauri/src/services/payment_service/repository.rs`

### Jika task tentang superadmin / managed radius / platform health

Urutan baca:

1. `src/routes/superadmin/+layout.svelte`
2. `src/routes/superadmin/+page.svelte`
3. `src/lib/api/superadmin.ts`
4. `src-tauri/src/http/superadmin.rs`
5. `src-tauri/src/services/system_service.rs`
6. `src-tauri/src/services/managed_radius_service.rs`

## Runtime Entry Points

### Frontend Entry

- `src/routes/+layout.ts`
  - Menonaktifkan SSR, jadi frontend berjalan sebagai SPA.
- `src/routes/+layout.svelte`
  - Boot aplikasi.
  - Menjalankan `checkAuth()`.
  - Inisialisasi settings/logo/i18n.
  - Menentukan install flow.
  - Menangani maintenance mode.
  - Membuka WebSocket bila user authenticated.
- `src/hooks.ts`
  - Me-rewrite custom domain ke route tenant internal `/:tenant/...`.
  - Menormalisasi legacy path seperti `/isp-management/...`.

### Desktop Entry

- `src-tauri/src/main.rs`
  - Hanya memanggil `saas_tauri_lib::run()`.
- `src-tauri/src/lib.rs`
  - Tauri builder.
  - Load `.env`.
  - Setup plugin Tauri.
  - Menjalankan `bootstrap::app::initialize_backend(...)`.
  - Register seluruh `invoke_handler` untuk mode desktop.

### Standalone Server Entry

- `src-tauri/src/bin/server.rs`
  - Init logging.
  - Load `.env`.
  - Init DB + seed defaults.
  - Bangun seluruh service.
  - Start Axum HTTP server via `http::start_server(...)`.

## Frontend Layer Map

### Route Skeleton

- Public/root:
  - `src/routes/+page.svelte`
  - `src/routes/login/+page.svelte`
  - `src/routes/install/+page.svelte`
  - `src/routes/register/+page.svelte`
  - `src/routes/forgot-password/...`
- Tenant app:
  - `src/routes/[tenant]/(app)/+layout.svelte`
  - Admin area: `src/routes/[tenant]/(app)/admin/...`
  - Customer portal: `src/routes/[tenant]/(app)/dashboard/...`
- Superadmin:
  - `src/routes/superadmin/...`

### Shared Frontend Control Points

- `src/lib/stores/auth.ts`
  - Menyimpan token/user/tenant.
  - `checkAuth()` memvalidasi token, refresh user, refresh tenant, dan mempertahankan session saat backend outage sementara.
- `src/lib/stores/websocket.ts`
  - Koneksi ke `/api/ws`.
  - Mengubah event WS menjadi refresh auth/notification/UI sync.
- `src/lib/api/core.ts`
  - Satu command map untuk endpoint HTTP.
  - Memilih HTTP API vs invoke/bridge Tauri.
- `src/lib/utils/apiUrl.ts`
  - Menentukan base URL:
    - `VITE_API_URL`
    - browser origin + `/api`
    - fallback `http://localhost:3000/api`

### Tenant Shell

- `src/routes/[tenant]/(app)/+layout.svelte`
  - Guard auth.
  - Guard tenant/domain canonicalization.
  - Guard RBAC admin path.
  - Memasang sidebar/topbar/banner.

## Backend Layer Map

### HTTP Composition

File utama: `src-tauri/src/bootstrap/http.rs`

Tanggung jawab:

- Membangun `AppState`
- Menjalankan background jobs:
  - rate limiter cleanup
  - security config refresh
  - IP block cleanup
  - alerting metrics loop
- Dynamic CORS dari env + `tenants.custom_domain`
- Register semua route `/api/...`

Route groups utama:

- `/api/auth/*`
- `/api/users/*`
- `/api/superadmin/*`
- `/api/support/*`
- `/api/plans/*`
- `/api/payment/*`
- `/api/notifications/*`
- `/api/email-outbox/*`
- `/api/admin/mikrotik/*`
- `/api/announcements/*`
- `/api/customers/*`
- `/api/admin/work-orders/*`
- `/api/admin/pppoe/*`
- `/api/admin/pppoe/mixradius/*`
- `/api/admin/isp-packages/*`
- `/api/admin/network-mapping/*`
- `/api/settings/*`
- `/api/roles`, `/api/permissions`
- `/api/ws`
- `/api/backups/*`
- `/api/storage/*`
- `/api/public/*`

### Service Construction Order

File utama:

- Desktop: `src-tauri/src/bootstrap/app.rs`
- Server: `src-tauri/src/bin/server.rs`

Urutan dependensi praktis:

1. DB init + seed
2. `PlanService`
3. `AuditService`
4. `RoleService`
5. `SettingsService`
6. `EmailService`
7. `AuthService`
8. Domain services lain:
   - `UserService`
   - `PppoeService`
   - `IspPackageService`
   - `NetworkMappingService`
   - `TeamService`
   - `SystemService`
   - `StorageService`
   - `EmailOutboxService`
   - `NotificationService`
   - `CustomerService`
   - `PaymentService`
   - `BackupService`
   - `MikrotikService`
   - `ManagedRadiusService`

## Core Domain Flows

### 1. Auth / Session / Tenant Resolution

Flow:

1. UI trigger dari `src/routes/+layout.svelte` atau `src/routes/login/+page.svelte`
2. `src/lib/stores/auth.ts`
3. `src/lib/api/core.ts`
4. `src-tauri/src/http/auth.rs`
5. `src-tauri/src/services/auth_service/mod.rs`
6. `src-tauri/src/services/auth_service/core.rs`
7. `src-tauri/src/services/auth_service/repository.rs`
8. DB tables seperti `users`, `sessions`, `settings`, `tenants`, `roles/permissions`

Catatan penting:

- Session aktif divalidasi terhadap tabel `sessions`, bukan hanya JWT.
- Expiry session diperpanjang secara sliding saat user aktif.
- Auth settings di-cache 60 detik.
- `get_current_user` mengembalikan user yang sudah diperkaya role/permission.

### 2. Customer / Portal / Subscription / Work Order

Flow admin:

1. Halaman admin customer di `src/routes/[tenant]/(app)/admin/customers/...`
2. `src/lib/api/customers.ts`
3. `src-tauri/src/http/customers.rs`
4. `src-tauri/src/services/customer_service/core.rs`
5. `src-tauri/src/services/customer_service/repository.rs`
6. DB: `customers`, `customer_locations`, `customer_subscriptions`, portal user tables, work order tables

Flow portal customer:

1. `src/routes/[tenant]/(app)/dashboard/...`
2. Endpoint `/api/customers/portal/*`
3. `CustomerService` branch `list_my_*`, `create_my_*`, `get_portal_customer_id`

Query/navigation notes:

- `list_customers()` memakai filter tenant + search + pagination.
- Postgres memakai `COUNT(*) OVER()` untuk menghindari query count terpisah pada listing customer.
- Banyak operasi create/update disertai audit log dan validasi permission sebelum write.

### 3. Network Mapping / Coverage / Topology Workspace

Flow:

1. `src/routes/[tenant]/(app)/admin/network/map/+page.svelte`
2. `src/lib/components/network/networkMap*.ts|.svelte`
3. `src/lib/api/networkMapping.ts`
4. `src-tauri/src/http/network_mapping.rs`
5. `src-tauri/src/services/network_mapping_service/mod.rs`
6. `src-tauri/src/services/network_mapping_service/core.rs`
7. `src-tauri/src/services/network_mapping_service/repository.rs`
8. DB GIS/topology tables

Subflows utama:

- Nodes CRUD
- Links CRUD
- Zone CRUD
- Path compute
- Candidate node ranking
- Coverage check
- Asset sync ke topology
- Impact customer listing

Query/design notes:

- Listing node/link mendukung search, status/kind filter, bbox filter.
- Postgres query memakai `ST_Intersects` + `ST_MakeEnvelope` untuk spatial filtering.
- Path/link scoring menghitung distance, latency, utilization, loss, dan status penalty.

### 4. MixRadius Import Wizard

Flow:

1. `src/routes/[tenant]/(app)/admin/network/import/mixradius/+page.svelte`
2. `src/lib/components/network/mixradius/MixRadiusImportWizard.svelte`
3. `src/lib/api/mixradiusImport.ts`
4. `src-tauri/src/http/mixradius_import.rs`
5. `src-tauri/src/services/mixradius_import_service/mod.rs`
6. `src-tauri/src/services/mixradius_import_executor.rs`
7. `src-tauri/src/services/mixradius_sql_parser.rs`
8. DB tables `mixradius_import_*`

Subflow:

1. Upload/local path diterima handler
2. `stage_backup()` membaca file dan mencatat batch
3. Parser memecah backup SQL
4. Parsed rows disimpan ke staging tables
5. `build_preview()` membangun preview + conflict state
6. `execute_preview()` menjalankan import via executor

Storage/IO notes:

- Ada file read langsung dari path lokal saat staging backup.
- Import dibangun batch-based dengan status/progress JSON tersimpan di DB.

### 5. Payment / Invoice / Billing Collection

Flow admin:

1. Halaman admin invoice/billing
2. `src/lib/api/payment.ts`
3. `src-tauri/src/http/payment.rs`
4. `src-tauri/src/services/payment_service/mod.rs`
5. `src-tauri/src/services/payment_service/core.rs`
6. `src-tauri/src/services/payment_service/repository.rs`
7. DB `invoices`, `bank_accounts`, reminder/collection logs, subscription tables

Flow portal:

1. Portal invoice/order page
2. Handler payment mengecek read scope:
   - billing admin
   - customer portal owner

Background flow:

1. `PaymentService::start_customer_invoice_scheduler()`
2. Generate due invoices
3. Jalankan billing collection
4. Suspend/resume PPPoE bila policy billing mengharuskan

Design notes:

- Service menghitung FX rate, invoice creation, Midtrans status transition, reminder, collection run.
- Ada guard untuk mencegah duplicate/downgrade transisi status pembayaran.

### 6. Superadmin / Managed Radius / System Health

Flow:

1. `src/routes/superadmin/...`
2. `src/lib/api/superadmin.ts`
3. `src-tauri/src/http/superadmin.rs`
4. `src-tauri/src/services/system_service.rs`
5. `src-tauri/src/services/managed_radius_service.rs`
6. DB tenant/platform/global managed-radius tables

Subdomain/platform scope:

- tenant lifecycle CRUD
- managed radius global servers/assignments/mappings
- platform audit
- system diagnostics/health

## Storage / File / Backup Flow

### Storage

- HTTP handlers:
  - `src-tauri/src/http/storage.rs`
- Core service:
  - `src-tauri/src/services/storage_service.rs`

Subflow:

1. Request upload/list/delete/content
2. Storage service validasi plan/tenant scope
3. File metadata di DB
4. File bytes di filesystem app data dir

### Backup

- HTTP handlers: `src-tauri/src/http/backup.rs`
- Core service: `src-tauri/src/services/backup.rs`
- Scheduler: `services::backup::BackupScheduler`

## Realtime / Background Jobs

### WebSocket

- Endpoint: `/api/ws`
- Handler: `src-tauri/src/http/websocket.rs`
- Frontend consumer: `src/lib/stores/websocket.ts`

Event consumers utama:

- notification updates
- role/permission changes
- maintenance mode changes
- support ticket activity

### Scheduled / Polling Services

- `PaymentService::start_customer_invoice_scheduler()`
- `CustomerService::start_installation_sla_scheduler()`
- `EmailOutboxService::start_sender()`
- `BackupScheduler::start()`
- `AnnouncementScheduler::start()`
- `MikrotikService::start_poller()`
- HTTP bootstrap background loops untuk security config, rate limit cleanup, alert checks, CORS refresh

## Database Navigation

### Starting Points

- DB feature selection: `src-tauri/src/db/connection/mod.rs`
- Migration runner: `src-tauri/src/bin/migrate.rs`
- Seed runner: `src-tauri/src/bin/seed.rs`

### Migration Themes

Urutan evolusi schema yang terlihat dari folder `src-tauri/migrations/`:

- baseline auth/settings/tenants
- support tickets
- announcements/email outbox
- customer + PPPoE
- MikroTik telemetry/alerts/incidents
- ISP packages + billing
- customer subscriptions + registration invites
- installation work orders
- network mapping foundation + expansions
- managed radius foundation/refactor
- MixRadius import foundation

### DB Cost Notes

Query yang patut dijadikan acuan saat menambah fitur baru:

- customer listing: tenant filter + pagination + single-pass total count
- network mapping listing: tenant filter + optional bbox spatial filter
- auth/session: token lookup langsung pada session aktif, bukan scan role/user berlapis
- CORS refresh: query ringan periodik hanya ke `tenants.custom_domain`

Saat mengubah query berat:

- pertahankan filter berbasis tenant sedini mungkin
- hindari N+1 pada listing customer/subscription/topology
- pastikan operasi scheduler/batch berbasis chunk atau scope tenant

## Directory Guide

### `src/`

- SvelteKit app shell, routes, stores, API client, UI components

### `src/lib/components/network/`

- Area frontend paling kompleks
- Workspace topology, map canvas, CRUD helper, wallboard, MixRadius wizard

### `src-tauri/src/http/`

- HTTP handlers per domain

### `src-tauri/src/services/`

- Business logic utama
- Beberapa domain besar sudah dipecah per submodule:
  - `auth_service/`
  - `customer_service/`
  - `network_mapping_service/`
  - `payment_service/`
  - `mixradius_import_service/`

### `src-tauri/src/models/`

- DTO/model DB/API lintas domain

### `src-tauri/migrations/`

- Evolusi schema Postgres utama

### `deploy/`

- Artefak ops untuk systemd + FreeRADIUS

### `docs/`

- Design docs, implementation plan, blueprint domain
- Referensi terbaik sebelum mengubah domain besar:
  - `docs/network-mapping/*`
  - `docs/superpowers/specs/*`
  - `docs/superpowers/plans/*`

## Practical Trace Recipes

### Cari bug UI admin page

1. route page
2. imported component/helper
3. API module yang dipanggil
4. HTTP handler domain
5. service method yang dipanggil handler
6. repository/query block yang dipakai

### Cari bug data salah tenant

1. handler `tenant_and_claims(...)`
2. service permission gate
3. query `WHERE tenant_id = ...`
4. frontend tenant prefix/domain resolver

### Cari bug auth/session expiry

1. `src/routes/+layout.svelte`
2. `src/lib/stores/auth.ts` `checkAuth()`
3. `src-tauri/src/http/auth.rs`
4. `AuthService::validate_token()`
5. tabel `sessions`

### Cari bug network map

1. `admin/network/map/+page.svelte`
2. helper `networkMapData`, `networkMapCrud`, `networkMapActions`
3. `http/network_mapping.rs`
4. method terkait di `network_mapping_service/mod.rs`
5. query spatial / repository terkait

### Cari bug import MixRadius

1. wizard component
2. `lib/api/mixradiusImport.ts`
3. `http/mixradius_import.rs`
4. `MixradiusImportService::{stage_backup, build_preview, execute_preview}`
5. `MixradiusImportExecutor`

## Non-Primary But Relevant Files

- `package.json`
  - script dev/build/check/lint/test/tauri/server build-run
- `vite.config.js`
  - host/CORS allowlist untuk SvelteKit dev
- `docker-compose.yml`
  - local Postgres only
- `scripts/check-i18n.mjs`
  - audit i18n consistency
- `scripts/copy-pdfjs-assets.mjs`
  - postinstall static asset sync

## Update Rule

Perbarui `SYSTEM_MAP.md` jika salah satu berubah:

- entrypoint app
- route group utama
- service utama per domain
- lokasi storage/DB flow
- file/domain baru yang menjadi jalur utama runtime
