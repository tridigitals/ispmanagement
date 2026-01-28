# 🚀 SaaS Boilerplate - Rust + Tauri + SvelteKit

A production-ready, multi-tenant SaaS boilerplate built with **Rust**, **Tauri 2**, and **SvelteKit 5**.

## ✨ Features

### 🔐 Authentication & Security
- JWT authentication with refresh tokens
- Password hashing with Argon2
- Two-Factor Authentication (TOTP & Email OTP)
- Password policy enforcement (min length, special chars, uppercase, numbers)
- Brute force protection with account lockout
- Email verification & Password reset
- Session management

### 👥 Multi-Tenancy
- Tenant isolation with slug-based routing
- Custom domain support per tenant
- Tenant-specific settings

### 🛡️ Authorization
- Role-Based Access Control (RBAC)
- Granular permissions system
- Admin/User/Custom roles

### 📧 Email Service
- SMTP support (via Lettre)
- Resend API integration
- SendGrid API integration
- Custom Webhook support

### 🔔 Notifications
- Real-time WebSocket notifications
- In-app notification center

### 💳 Billing & Subscription
- Plans management
- Subscription tracking
- Invoice generation

### 📁 File Storage
- Local file storage
- AWS S3 compatible storage
- File manager UI

### 📊 Audit & Logging
- Comprehensive audit logging
- Action tracking per user

### 🌐 Internationalization
- i18n support (English & Indonesian)
- Easy to add more languages

---

## 🛠️ Tech Stack

| Layer | Technology |
|-------|------------|
| **Desktop Runtime** | Tauri 2 |
| **Backend** | Rust + Axum |
| **Frontend** | SvelteKit 5 + TypeScript |
| **Database** | PostgreSQL (default) / SQLite |
| **Authentication** | JWT + Argon2 |
| **2FA** | TOTP (totp-rs) |
| **Email** | Lettre (SMTP) + Reqwest (API) |
| **Storage** | AWS S3 SDK |

---

## 📋 Prerequisites

- **Node.js** >= 18
- **Rust** >= 1.75
- **Docker** (for PostgreSQL) or SQLite

---

## 🚀 Quick Start

### 1. Clone & Install

```bash
git clone <repo-url> my-saas-app
cd my-saas-app
npm install
```

### 2. Environment Setup

```bash
cp .env.example .env
```

Edit `.env` with your configuration.

### 3. Start Database (PostgreSQL)

```bash
docker-compose up -d
```

### 4. Run Development Server

```bash
npm run tauri dev
```

The app will open automatically. First run will show the **Installation Wizard**.

---

## ⚙️ Environment Variables

### Required

| Variable | Description | Example |
|----------|-------------|---------|
| `DATABASE_URL` | Database connection string | `postgres://user:pass@localhost:5433/db` |
| `PORT` | HTTP server port | `3000` |
| `CORS_ALLOWED_ORIGINS` | Allowed CORS origins (comma-separated) | `http://localhost:5173` |

### PostgreSQL (Docker)

| Variable | Description | Default |
|----------|-------------|---------|
| `POSTGRES_USER` | Database user | `saas_user` |
| `POSTGRES_PASSWORD` | Database password | `saas_password` |
| `POSTGRES_DB` | Database name | `saas_db` |
| `POSTGRES_PORT` | Exposed port | `5433` |

### For SQLite Mode

```env
DATABASE_URL=sqlite:./saas_app.db?mode=rwc
```

Run with:
```bash
npm run tauri dev -- -- --features sqlite --no-default-features
```

---

## 📁 Project Structure

```
├── src/                    # SvelteKit Frontend
│   ├── lib/
│   │   ├── api/           # API client
│   │   ├── components/    # Reusable UI components
│   │   ├── i18n/          # Internationalization
│   │   ├── stores/        # Svelte stores
│   │   └── utils/         # Utilities
│   └── routes/
│       ├── [tenant]/      # Tenant-scoped routes
│       │   └── (app)/     # Protected app routes
│       │       ├── admin/ # Admin panel
│       │       └── dashboard/
│       ├── superadmin/    # Super admin panel
│       ├── login/         # Auth pages
│       ├── register/
│       └── install/       # Installation wizard
│
├── src-tauri/              # Rust Backend
│   └── src/
│       ├── commands/      # Tauri commands
│       ├── db/            # Database connection
│       ├── http/          # Axum HTTP routes
│       ├── models/        # Data models
│       └── services/      # Business logic
│
├── static/                 # Static assets
├── docker-compose.yml      # PostgreSQL container
└── .env.example           # Environment template
```

---

## 🗄️ Database Modes

### PostgreSQL (Default - Recommended)

```bash
# Start PostgreSQL container
docker-compose up -d

# Run app
npm run tauri dev
```

### SQLite (Development/Testing)

```bash
npm run tauri dev -- -- --features sqlite --no-default-features
```

---

## 🔗 Default Ports

| Service | Port | Description |
|---------|------|-------------|
| Tauri Dev | `1420` | SvelteKit dev server |
| HTTP API | `3000` | Axum HTTP server |
| PostgreSQL | `5433` | Database |
| WebSocket | `3000/ws` | Real-time notifications |

---

## 👤 First Run - Installation

On first run, the **Installation Wizard** will appear:

1. **Create Super Admin** - Set up the main admin account
2. **Configure Settings** - Basic app settings
3. **Done!** - Redirect to login

---

## 🎨 UI Components

Available in `src/lib/components/`:

| Component | Description |
|-----------|-------------|
| `Table.svelte` | Data table with pagination, sorting |
| `Modal.svelte` | Modal dialogs |
| `ConfirmDialog.svelte` | Confirmation dialogs |
| `FileManager.svelte` | Full file manager |
| `Sidebar.svelte` | Navigation sidebar |
| `Topbar.svelte` | Top navigation bar |
| `Input.svelte` | Form input component |
| `Select.svelte` | Dropdown select |
| `Pagination.svelte` | Pagination controls |
| `StatsCard.svelte` | Dashboard stat cards |
| `NotificationDropdown.svelte` | Notification bell |
| `Lightbox.svelte` | Image lightbox |

---

## 🔧 Customization

### Adding a New Language

1. Create locale file: `src/lib/i18n/locales/de.json`
2. Register in `src/lib/i18n/index.ts`

### Adding a New Route

1. Create folder in `src/routes/[tenant]/(app)/your-route/`
2. Add `+page.svelte`
3. Add menu item in `Sidebar.svelte`

### Adding a New API Endpoint

1. Create handler in `src-tauri/src/http/`
2. Register route in `src-tauri/src/http/mod.rs`

---

## 📝 API Endpoints

### Authentication
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/auth/login` | User login |
| POST | `/api/auth/register` | User registration |
| POST | `/api/auth/logout` | Logout |
| GET | `/api/auth/validate` | Validate token |
| POST | `/api/auth/forgot-password` | Request password reset |
| POST | `/api/auth/reset-password` | Reset password |

### Users
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/users` | List users |
| GET | `/api/users/:id` | Get user |
| PUT | `/api/users/:id` | Update user |
| DELETE | `/api/users/:id` | Delete user |

### Tenants
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/tenants` | List tenants |
| POST | `/api/tenants` | Create tenant |
| GET | `/api/tenants/:id` | Get tenant |
| PUT | `/api/tenants/:id` | Update tenant |

### Settings
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/settings` | Get all settings |
| PUT | `/api/settings` | Update settings |

---

## 🚢 Production Build

```bash
# Build for production
npm run tauri build
```

Output will be in `src-tauri/target/release/`.

---

## 📜 License

MIT

---

## 🤝 Notes

- Database tables are auto-created on first run
- JWT secret is auto-generated if not set
- Default session timeout: 24 hours
- WebSocket reconnects automatically on disconnect
