# Task: Feature-Based Subscription Plan System

## Progress Checklist

### Database
- [x] Create PostgreSQL migrations/logic for plans tables (in `connection.rs`)
- [x] Create SQLite migrations/logic for plans tables (in `connection.rs`)
- [x] Add seed data for default plans and features (in `connection.rs`)
- [x] Phase 7: Fixing Broken Pages (Plans, Audit Logs)
- [x] Phase 8: Layout & Breakpoint Synchronization
- [x] Phase 9: Audit Logs Table Design Optimization
    - [x] Create condensed multi-line column layout
    - [x] Merge User and Tenant info
    - [x] Merge Action and Resource info
    - [x] Optimize widths to eliminate horizontal scroll

### Backend (Rust)
- [x] Create Plan models (`src-tauri/src/models/plan.rs`)
- [x] Create PlanService (`src-tauri/src/services/plan_service.rs`)
- [x] Create Tauri commands (`src-tauri/src/commands/plans.rs`)
- [x] Create HTTP endpoints (`src-tauri/src/http/plans.rs`)
- [x] Wire up in `lib.rs` and module exports

### Frontend (SvelteKit)
- [x] Update API client with plans methods (`src/lib/api/client.ts`)
- [x] Create Plans management page (`src/routes/superadmin/plans/`)
- [ ] Create Features management page (Partially integrated into Plans page)
- [x] Add navigation to Superadmin layout
- [x] Update Tenants page with plan assignment (`src/routes/superadmin/tenants/+page.svelte`)
- [ ] Implement Plan upgrade/downgrade UI for Tenants

### Verification
- [ ] Test plan CRUD operations
- [ ] Test feature assignment
- [ ] Test tenant subscription
- [ ] Test feature access checking