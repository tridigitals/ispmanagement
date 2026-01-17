# Task: Feature-Based Subscription Plan System

## Progress Checklist

### Database
- [/] Create PostgreSQL migration for plans tables
- [ ] Create SQLite migration for plans tables
- [ ] Add seed data for default plans and features

### Backend (Rust)
- [ ] Create Plan models (`src-tauri/src/models/plan.rs`)
- [ ] Create PlanService (`src-tauri/src/services/plan_service.rs`)
- [ ] Create Tauri commands (`src-tauri/src/commands/plans.rs`)
- [ ] Create HTTP endpoints (`src-tauri/src/http/plans.rs`)
- [ ] Wire up in `lib.rs` and module exports

### Frontend (SvelteKit)
- [ ] Update API client with plans methods
- [ ] Create Plans management page
- [ ] Create Features management page
- [ ] Add navigation to Superadmin layout
- [ ] Update Tenants page with plan assignment

### Verification
- [ ] Test plan CRUD operations
- [ ] Test feature assignment
- [ ] Test tenant subscription
- [ ] Test feature access checking
